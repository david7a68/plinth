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

use std::sync::mpsc::{Receiver, Sender};

use arrayvec::ArrayVec;
use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS, QUERY_DISPLAY_CONFIG_FLAGS,
    },
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    Graphics::{
        Dxgi::{IDXGIOutput, DXGI_OUTPUT_DESC},
        Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW},
    },
};

use crate::{
    frame::{FrameId, FramesPerSecond},
    limits::MAX_WINDOWS,
    platform::handle_pool::{Handle, HandlePool},
};

use super::AppContextImpl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VsyncCookie(Handle<()>);

pub enum VsyncRequest<T: From<VSyncReply>> {
    /// Register the sender with the vsync thread.
    Register(Sender<T>),
    /// Deregister the sender with the vsync thread. All pending requests will
    /// be cancelled. Dropping the receiver will also cause deregistration.
    Deregister(VsyncCookie),
    /// Requests that the vsync thread cancel any pending requests. You will
    /// have to send a new request to be notified of vsync events.
    Idle(VsyncCookie),
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
    AtFrame(VsyncCookie, FrameId),
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
    AtFrameRate(VsyncCookie, FramesPerSecond),
}

#[derive(Clone, Copy, Debug)]
pub enum VSyncReply {
    /// Sent once a sender is registered with the vsync thread.
    Registered {
        cookie: VsyncCookie,
        // todo: this is a bad name
        composition_rate: FramesPerSecond,
    },
    /// Send when a client-requested VSync event occurs.
    VSync {
        frame: FrameId,
        /// The rate at which the vsync thread is sending notifications. This is
        /// `None` if the notification was produced by a `VSyncRequest::VSyncAt`
        /// request.
        rate: Option<FramesPerSecond>,
    },
    /// Sent when the main monitor's refresh rate changes.
    DeviceUpdate {
        /// The vblank when the update occurred. All subsequent vblanks will
        /// occur with the new composition rate.
        frame: FrameId,
        /// The new composition rate.
        // todo: this is a bad name
        composition_rate: FramesPerSecond,
    },
}

struct Client<T> {
    sender: Sender<T>,
    mode: Mode,
}

enum Mode {
    Idle,
    AtFrame(FrameId),
    ForFps {
        interval: u16,
        requested: FramesPerSecond,
        next: FrameId,
    },
}

// :assert_max_windows_u16:
const _: () = assert!(MAX_WINDOWS <= u16::MAX as usize);

pub struct VsyncThread<'a, T: From<VSyncReply>> {
    context: &'a AppContextImpl,
    request_receiver: &'a Receiver<VsyncRequest<T>>,

    clients: HandlePool<Client<T>, MAX_WINDOWS, ()>,

    counter: u64,
    main_output: IDXGIOutput,
    main_monitor: HMONITOR,
    composition_rate: FramesPerSecond,
}

impl<'a, T: From<VSyncReply> + 'static> VsyncThread<'a, T> {
    pub fn new(
        context: &'a AppContextImpl,
        request_receiver: &'a Receiver<VsyncRequest<T>>,
    ) -> Self {
        let clients = HandlePool::new();

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
        let mut delivery_errors = ArrayVec::<Handle<()>, MAX_WINDOWS>::new();

        let composition_rate_changed = self.update_composition_rate();

        if composition_rate_changed {
            let msg = VSyncReply::DeviceUpdate {
                frame: current_frame,
                composition_rate: self.composition_rate,
            };

            for (handle, client) in self.clients.iter() {
                Self::try_deliver(handle, client, msg, &mut delivery_errors);
            }

            for (_, client) in self.clients.iter_mut() {
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

        // handle any requests
        while let Ok(request) = self.request_receiver.try_recv() {
            match request {
                VsyncRequest::Register(sender) => {
                    let Some((handle, client)) = self.clients.insert(Client {
                        sender,
                        mode: Mode::Idle,
                    }) else {
                        tracing::warn!(
                            "Attempt to register too many vsync clients. Ignoring request."
                        );

                        continue;
                    };

                    let cookie = VsyncCookie(unsafe { handle.retype() });

                    let message = VSyncReply::Registered {
                        cookie,
                        composition_rate: self.composition_rate,
                    };

                    Self::try_deliver(handle, client, message, &mut delivery_errors);
                }
                VsyncRequest::Deregister(cookie) => {
                    if self.clients.remove(cookie.0).is_none() {
                        tracing::warn!("Attempt to deregister invalid vsync client. Ignoring.");
                    }
                }
                VsyncRequest::Idle(cookie) => {
                    if let Some(client) = self.clients.get_mut(cookie.0) {
                        client.mode = Mode::Idle;
                    } else {
                        tracing::warn!("Attempt to idle invalid vsync client. Ignoring.");
                    }
                }
                VsyncRequest::AtFrame(cookie, frame) => {
                    if let Some(client) = self.clients.get_mut(cookie.0) {
                        client.mode = Mode::AtFrame(frame);
                    } else {
                        tracing::warn!("Attempt to idle invalid vsync client. Ignoring.");
                    }
                }
                VsyncRequest::AtFrameRate(cookie, rate) => {
                    if let Some(client) = self.clients.get_mut(cookie.0) {
                        client.mode = Mode::ForFps {
                            interval: interval_from_rate(rate, self.composition_rate),
                            next: FrameId(0),
                            requested: rate,
                        };
                    } else {
                        tracing::warn!("Attempt to idle invalid vsync client. Ignoring.");
                    }
                }
            }
        }

        // dispatch any requests that are due
        for (handle, client) in self.clients.iter_mut() {
            Self::try_advance(
                handle,
                client,
                current_frame,
                self.composition_rate,
                &mut delivery_errors,
            );
        }

        for handle in delivery_errors {
            self.clients.remove(handle);
        }

        unsafe { self.main_output.WaitForVBlank() }.unwrap();
        self.counter += 1;
    }

    fn try_deliver(
        handle: Handle<()>,
        client: &Client<T>,
        notification: VSyncReply,
        errors: &mut ArrayVec<Handle<()>, MAX_WINDOWS>,
    ) {
        let delivered = client.sender.send(notification.into()).is_ok();

        if !delivered {
            tracing::warn!("Attempt to deliver vsync notification failed. Reason: channel closed. Removing client.");

            if !errors.contains(&handle) {
                errors.push(handle);
            }
        }
    }

    fn try_advance(
        handle: Handle<()>,
        client: &mut Client<T>,
        frame: FrameId,
        composition_rate: FramesPerSecond,
        errors: &mut ArrayVec<Handle<()>, MAX_WINDOWS>,
    ) {
        let (is_due, rate) = match &mut client.mode {
            Mode::Idle => (false, None),
            Mode::AtFrame(target) => (*target == frame, None),
            Mode::ForFps { interval, next, .. } => {
                let is_due = frame >= *next;

                *next += {
                    // skip frames if we're behind
                    let diff = frame.0.saturating_sub(next.0);
                    let catch_up = (diff as f64 / *interval as f64).ceil() as u64;
                    *interval as u64 * catch_up
                };

                // branchless: if is_due { *next += *interval as u64 }
                *next += *interval as u64 * is_due as u64;

                debug_assert!(next.0 >= frame.0);
                (is_due, Some(*interval))
            }
        };

        let rate = rate.map(|interval| composition_rate / interval as f64);

        if is_due {
            Self::try_deliver(handle, client, VSyncReply::VSync { frame, rate }, errors);
        }
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
