use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use windows::{
    core::ComInterface,
    Win32::{
        Devices::Display::{
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
            DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
            QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_REFRESH_RATE_AWARE, QUERY_DISPLAY_CONFIG_FLAGS,
        },
        Foundation::{ERROR_INSUFFICIENT_BUFFER, HWND},
        Graphics::{
            Direct3D12::{D3D12GetDebugInterface, ID3D12Debug1, ID3D12Debug5},
            DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
            Dxgi::{
                CreateDXGIFactory2, IDXGIFactory2, IDXGIOutput, DXGI_CREATE_FACTORY_DEBUG,
                DXGI_OUTPUT_DESC,
            },
            Gdi::{GetMonitorInfoW, MONITORINFO, MONITORINFOEXW},
        },
    },
};

use crate::{
    graphics::{GraphicsConfig, RefreshRate},
    platform::dx12,
    time::FramesPerSecond,
    window::{WindowEventHandler, WindowSpec},
};

use super::{swapchain::Swapchain, window::spawn_window};

#[derive(Debug)]
pub(super) enum AppMessage {
    WindowCreated,
    WindowClosed,
}

pub struct ApplicationImpl {
    context: AppContextImpl,
    receiver: Receiver<AppMessage>,
}

impl ApplicationImpl {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        // TODO: this bound is nonsense. actually figure out what it should be.
        let (sender, receiver) = std::sync::mpsc::channel();

        let context = AppContextImpl::new(graphics, sender);

        Self { context, receiver }
    }

    pub fn context(&self) -> AppContextImpl {
        self.context.clone()
    }

    pub fn spawn_window<W, F>(&self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        spawn_window(self.context(), spec, constructor);
    }

    pub fn run(&mut self) {
        let mut num_windows = 0;

        while let Ok(msg) = self.receiver.recv() {
            match msg {
                AppMessage::WindowCreated => num_windows += 1,
                AppMessage::WindowClosed => num_windows -= 1,
            }

            // This is redundant so long as only windows hold AppContexts.
            if num_windows == 0 {
                break;
            }
        }
    }
}

impl Drop for ApplicationImpl {
    fn drop(&mut self) {
        // wait for graphics thread to exit
    }
}

#[derive(Clone)]
pub struct AppContextImpl {
    pub dxgi: IDXGIFactory2,
    pub dx12: Arc<dx12::Device>,
    pub compositor: IDCompositionDevice,
    pub main_output: IDXGIOutput,
    pub(super) sender: Sender<AppMessage>,
}

impl AppContextImpl {
    fn new(config: &GraphicsConfig, sender: Sender<AppMessage>) -> Self {
        let dxgi: IDXGIFactory2 = {
            let mut dxgi_flags = 0;

            if config.debug_mode {
                let mut controller: Option<ID3D12Debug1> = None;
                unsafe { D3D12GetDebugInterface(&mut controller) }.unwrap();

                if let Some(controller) = controller {
                    tracing::info!("Enabling D3D12 debug layer");
                    unsafe { controller.EnableDebugLayer() };
                    unsafe { controller.SetEnableGPUBasedValidation(true) };

                    if let Ok(controller) = controller.cast::<ID3D12Debug5>() {
                        unsafe { controller.SetEnableAutoName(true) };
                    }
                } else {
                    tracing::warn!("Failed to enable D3D12 debug layer");
                }

                dxgi_flags |= DXGI_CREATE_FACTORY_DEBUG;
            }

            unsafe { CreateDXGIFactory2(dxgi_flags) }.unwrap()
        };

        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();

        let main_output = {
            let adapter0 = unsafe { dxgi.EnumAdapters(0) }.unwrap();
            unsafe { adapter0.EnumOutputs(0) }.unwrap()
        };

        let dx12 = dx12::Device::new(&dxgi, config);

        Self {
            dxgi,
            dx12,
            compositor,
            main_output,
            sender,
        }
    }

    pub fn composition_rate(&self) -> FramesPerSecond {
        // todo: make use of max refresh rate
        get_output_refresh_rate(&self.main_output).now
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        spawn_window(self.clone(), spec, constructor);
    }

    pub fn create_swapchain(&self, hwnd: HWND) -> Swapchain {
        Swapchain::new(&self.dxgi, &self.compositor, self.dx12.queue(), hwnd)
    }

    pub fn wait_for_main_monitor_vblank(&self) {
        unsafe { self.main_output.WaitForVBlank() }.unwrap();
    }
}

unsafe impl Send for AppContextImpl {}
unsafe impl Sync for AppContextImpl {}

fn get_output_refresh_rate(output: &IDXGIOutput) -> RefreshRate {
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

    let max_refresh_rate = if windows_version::OsVersion::current()
        >= windows_version::OsVersion::new(10, 0, 0, 22000)
    {
        query_display_config(
            QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_REFRESH_RATE_AWARE,
            &mut paths,
            &mut modes,
        );
        get_output_refresh_rate_from_path(&monitor_info.szDevice, &paths)
    } else {
        refresh_rate
    };

    RefreshRate {
        min: FramesPerSecond(0.0),
        max: FramesPerSecond(max_refresh_rate),
        now: FramesPerSecond(refresh_rate),
    }
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
                if tries > 10 {
                    panic!("Failed to query display config (too many retries): {e:?}");
                }
                if e.code() == ERROR_INSUFFICIENT_BUFFER.into() {
                    tries += 1;
                } else {
                    panic!("Failed to query display config: {e:?}");
                }
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
