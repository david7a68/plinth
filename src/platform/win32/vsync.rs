//! Vsync interrupt handling.
//!
//! This module exposes a bidirectional message interface (a.k.a. a send and a
//! receive message pair) and a struct holding thread state. Vsync events are
//! tied to the main monitor (as Windows' compositor does), and are laid out on
//! a timeline. Client windows submit a request to be notified when a specific
//! vblank occurs, and must do so for every vblank they wish to be notified of.
//!
//! There are three kinds of requests:
//! - `VSyncAt`: Requests a notification when a specific vblank occurs.
//! - `VSyncForFps`: Requests a notification at a rate strictly at or above the
//!  specified rate (e.g. a request for 24 fps might be returned at 30 fps).
//! - `VSyncForInterval`: Requests a notification every `interval` vblanks.
//!
//! This... might be too much. I'm not sure. But I wanted to consolidate all the
//! frame timing stuff into one place instead of handling it per-window.

use std::sync::mpsc::Receiver;

use arrayvec::ArrayVec;
use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS, QUERY_DISPLAY_CONFIG_FLAGS,
    },
    Foundation::{ERROR_INSUFFICIENT_BUFFER, HWND, LPARAM, WPARAM},
    Graphics::{
        Dxgi::{IDXGIOutput, DXGI_OUTPUT_DESC},
        Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW},
    },
    UI::WindowsAndMessaging::PostMessageW,
};

use crate::{
    frame::{FrameId, FramesPerSecond},
    limits::MAX_WINDOWS,
};

use super::{
    window::{UM_COMPOSITION_RATE, UM_VSYNC},
    AppContextImpl,
};

pub enum VsyncRequest {
    /// Requests that the vsync thread cancel any pending requests. You will
    /// have to send a new request to be notified of vsync events.
    Idle(HWND),
    /// Requests that a VSync notification be sent when the specified vblank
    /// occurs. If the vblank has already passed, a reply will be sent
    /// immediately.
    ///
    /// `VSyncReply`s sent in response to this event will have a `rate` of
    /// `None`, since the notification is not tied to a specific refresh rate.
    ///
    /// ## Usage
    ///
    /// Use this if you need to update something at some time in the future.
    /// This is basically a timer aligned to the nearest vblank.
    AtFrame(HWND, FrameId),
    /// Requests that VSync notifications be sent at a rate strictly at or above
    /// the specified rate (e.g. a request for 24 fps might be returned at 30
    /// fps). The vsync thread will attempt to match the requested rate as
    /// closely as possible, but may not be able to do so. This will be
    /// dynamically updated if the main monitor's refresh rate changes, so
    /// frame-to-frame timings may change.
    ///
    /// ## Usage
    ///
    /// Use this for animations and other things that need to be smooth, but
    /// aren't sensitive to exact timing. This is basically the same as
    /// `VSyncForInterval`, but automatically recalculated when the main
    /// monitor's refresh rate changes.
    AtFrameRate(HWND, FramesPerSecond),
}

fn encode_reply_vsync(frame: FrameId, rate: Option<FramesPerSecond>) -> (WPARAM, LPARAM) {
    let wp = WPARAM(unsafe { std::mem::transmute(frame.0) });
    let lp = LPARAM(unsafe { std::mem::transmute(rate.unwrap_or_default().0.to_bits()) });
    (wp, lp)
}

pub fn decode_reply_vsync(wparam: WPARAM, lparam: LPARAM) -> (FrameId, Option<FramesPerSecond>) {
    let frame = FrameId(unsafe { std::mem::transmute(wparam.0) });
    let rate: f64 = unsafe { std::mem::transmute(lparam.0) };

    if rate == 0.0 {
        (frame, None)
    } else {
        (frame, Some(FramesPerSecond(rate)))
    }
}

fn encode_reply_device_update(
    frame: FrameId,
    composition_rate: FramesPerSecond,
) -> (WPARAM, LPARAM) {
    let wp = WPARAM(unsafe { std::mem::transmute(frame.0) });
    let lp = LPARAM(unsafe { std::mem::transmute(composition_rate.0.to_bits()) });
    (wp, lp)
}

pub fn decode_reply_device_update(wparam: WPARAM, lparam: LPARAM) -> (FrameId, FramesPerSecond) {
    let frame = FrameId(unsafe { std::mem::transmute(wparam.0) });
    let rate = FramesPerSecond(unsafe { std::mem::transmute(lparam.0) });
    (frame, rate)
}

struct Client {
    hwnd: HWND,
    mode: Mode,
}

enum Mode {
    AtFrame(FrameId),
    ForFps {
        interval: u16,
        requested: FramesPerSecond,
        next: FrameId,
    },
}

// :assert_max_windows_u16:
const _: () = assert!(MAX_WINDOWS <= u16::MAX as usize);

pub struct VsyncThread<'a> {
    context: &'a AppContextImpl,
    request_receiver: &'a Receiver<VsyncRequest>,

    clients: ArrayVec<Client, MAX_WINDOWS>,

    counter: u64,
    main_output: IDXGIOutput,
    main_monitor: HMONITOR,
    composition_rate: FramesPerSecond,
}

impl<'a> VsyncThread<'a> {
    pub fn new(context: &'a AppContextImpl, request_receiver: &'a Receiver<VsyncRequest>) -> Self {
        let clients = ArrayVec::new();

        let main_output = {
            let adapter0 = unsafe { context.inner.read().dxgi.EnumAdapters(0) }.unwrap();
            unsafe { adapter0.EnumOutputs(0) }.unwrap()
        };

        // todo: handle boosted clocks
        let composition_rate = get_output_refresh_rate(&main_output);

        let main_monitor = {
            let mut desc = DXGI_OUTPUT_DESC::default();
            unsafe { main_output.GetDesc(&mut desc) }.unwrap();
            desc.Monitor
        };

        Self {
            context,
            request_receiver,
            clients,
            counter: 0,
            main_output,
            main_monitor,
            composition_rate,
        }
    }

    fn update_composition_rate(&mut self) -> bool {
        let composition_rate: FramesPerSecond =
            if unsafe { self.context.inner.read().dxgi.IsCurrent() }.as_bool() {
                self.composition_rate
            } else {
                let (main_output, main_monitor) = {
                    let mut inner = self.context.inner.write();
                    inner.update_device();

                    let adapter0 = unsafe { inner.dxgi.EnumAdapters(0) }.unwrap();
                    let output = unsafe { adapter0.EnumOutputs(0) }.unwrap();

                    let mut desc = DXGI_OUTPUT_DESC::default();
                    unsafe { output.GetDesc(&mut desc) }.unwrap();
                    (output, desc.Monitor)
                };

                if main_monitor != self.main_monitor {
                    self.main_output = main_output;
                    self.main_monitor = main_monitor;
                    get_output_refresh_rate(&self.main_output);
                }

                get_output_refresh_rate(&self.main_output)
            };

        let changed = composition_rate != self.composition_rate;
        self.composition_rate = composition_rate;
        changed
    }

    /// Receives vsync requests, dispatches requests that are due, and waits for
    /// the next vblank of the main monitor.
    pub fn tick(&mut self) {
        let current_frame = FrameId(self.counter);
        let mut delivery_errors = ArrayVec::<usize, MAX_WINDOWS>::new();

        let composition_rate_changed = self.update_composition_rate();

        if composition_rate_changed {
            let (wp, lp) = encode_reply_device_update(current_frame, self.composition_rate);

            for (index, client) in self.clients.iter_mut().enumerate() {
                let r = unsafe { PostMessageW(client.hwnd, UM_COMPOSITION_RATE, wp, lp) };
                if r.is_err() {
                    delivery_errors.push(index);
                } else {
                    // todo: do we need to update next? -dz
                    if let Mode::ForFps {
                        interval,
                        requested,
                        next: _,
                    } = &mut client.mode
                    {
                        *interval = interval_from_rate(*requested, self.composition_rate);
                    }
                }
            }
        }

        // handle any requests
        while let Ok(request) = self.request_receiver.try_recv() {
            match request {
                VsyncRequest::Idle(hwnd) => {
                    match self.clients.binary_search_by_key(&hwnd.0, |c| c.hwnd.0) {
                        Ok(index) => {
                            self.clients.remove(index);
                        }
                        Err(_) => {} // no-op since the client is already idle
                    }
                }
                VsyncRequest::AtFrame(hwnd, frame) => {
                    let mode = Mode::AtFrame(frame);
                    match self.clients.binary_search_by_key(&hwnd.0, |c| c.hwnd.0) {
                        Ok(index) => self.clients[index].mode = mode,
                        Err(index) => self.clients.insert(index, Client { hwnd, mode }),
                    }
                }
                VsyncRequest::AtFrameRate(hwnd, rate) => {
                    let mode = Mode::ForFps {
                        interval: interval_from_rate(rate, self.composition_rate),
                        next: FrameId(0),
                        requested: rate,
                    };

                    match self.clients.binary_search_by_key(&hwnd.0, |c| c.hwnd.0) {
                        Ok(index) => self.clients[index].mode = mode,
                        Err(index) => self.clients.insert(index, Client { hwnd, mode }),
                    }
                }
            }
        }

        // dispatch any requests that are due
        for (index, client) in self.clients.iter_mut().enumerate() {
            // perf: this does not need to be an enum, could remove the branch -dz
            let (is_due, rate) = match &mut client.mode {
                Mode::AtFrame(target) => (*target <= current_frame, None),
                Mode::ForFps {
                    interval,
                    requested: _,
                    next,
                } => {
                    let is_due = current_frame >= *next;

                    *next += {
                        // skip frames if we're behind
                        let diff = current_frame.0.saturating_sub(next.0);
                        let catch_up = (diff as f64 / *interval as f64).ceil() as u64;
                        *interval as u64 * catch_up
                    };

                    // branchless version of `if is_due { *next += *interval as u64 }`
                    *next += *interval as u64 * is_due as u64;

                    debug_assert!(next.0 >= current_frame.0);
                    (is_due, Some(self.composition_rate / *interval as f64))
                }
            };

            if is_due {
                let (wp, lp) = encode_reply_vsync(current_frame, rate);
                let r = unsafe { PostMessageW(client.hwnd, UM_VSYNC, wp, lp) };

                if r.is_err() {
                    delivery_errors.push(index);
                }
            }
        }

        for index in delivery_errors.drain(..).rev() {
            self.clients.remove(index);
        }

        unsafe { self.main_output.WaitForVBlank() }.unwrap();
        self.counter += 1;
    }
}

#[tracing::instrument(skip(output))]
fn get_output_refresh_rate(output: &IDXGIOutput) -> FramesPerSecond {
    let monitor = {
        let mut desc = DXGI_OUTPUT_DESC::default();
        unsafe { output.GetDesc(&mut desc) }.unwrap();
        desc.Monitor
    };

    let monitor_info = {
        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.unwrap();
        info
    };

    let mut paths = vec![];
    let mut modes = vec![];

    query_display_config(QDC_ONLY_ACTIVE_PATHS, &mut paths, &mut modes);
    let refresh_rate = get_output_refresh_rate_from_path(&monitor_info.szDevice, &paths);

    // let max_refresh_rate = if windows_version::OsVersion::current()
    //     >= windows_version::OsVersion::new(10, 0, 0, 22000)
    // {
    //     query_display_config(
    //         QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_REFRESH_RATE_AWARE,
    //         &mut paths,
    //         &mut modes,
    //     );
    //     get_output_refresh_rate_from_path(&monitor_info.szDevice, &paths)
    // } else {
    //     refresh_rate
    // };

    // todo: properly handle boosted refresh clocks

    FramesPerSecond(refresh_rate)
}

fn query_display_config(
    flags: QUERY_DISPLAY_CONFIG_FLAGS,
    paths: &mut Vec<DISPLAYCONFIG_PATH_INFO>,
    modes: &mut Vec<DISPLAYCONFIG_MODE_INFO>,
) {
    let mut tries = 0;

    loop {
        let (mut n_paths, mut n_modes) = (0, 0);
        unsafe { GetDisplayConfigBufferSizes(flags, &mut n_paths, &mut n_modes) }.unwrap();

        if n_paths as usize > paths.capacity() {
            paths.reserve_exact(n_paths as usize - paths.capacity());
        }

        if n_modes as usize > modes.capacity() {
            modes.reserve_exact(n_modes as usize - modes.capacity());
        }

        let r = unsafe {
            QueryDisplayConfig(
                flags,
                &mut n_paths,
                paths.as_mut_ptr(),
                &mut n_modes,
                modes.as_mut_ptr(),
                None,
            )
        };

        match r {
            Ok(()) => unsafe {
                paths.set_len(n_paths as usize);
                modes.set_len(n_modes as usize);
                break;
            },
            Err(e) => {
                assert!(
                    tries <= 10,
                    "Failed to query display config (too many retries): {e:?}"
                );
                assert!(
                    e.code() == ERROR_INSUFFICIENT_BUFFER.into(),
                    "Failed to query display config: {e:?}"
                );

                tries += 1;
            }
        }
    }
}

fn get_output_refresh_rate_from_path(
    output_name: &[u16; 32],
    paths: &[DISPLAYCONFIG_PATH_INFO],
) -> f64 {
    for path in paths {
        let mut request = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };

        // cleanup: handle this error properly
        assert_eq!(
            unsafe { DisplayConfigGetDeviceInfo(&mut request.header) },
            0
        );

        if request.viewGdiDeviceName == *output_name {
            let numerator = path.targetInfo.refreshRate.Numerator;
            let denominator = path.targetInfo.refreshRate.Denominator;

            return numerator as f64 / denominator as f64;
        }
    }

    0.0
}

fn interval_from_rate(rate: FramesPerSecond, composition_rate: FramesPerSecond) -> u16 {
    (composition_rate.0 / rate.0).floor() as u16
}
