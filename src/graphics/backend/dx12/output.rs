use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_REFRESH_RATE_AWARE, QUERY_DISPLAY_CONFIG_FLAGS,
    },
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    Graphics::{
        Dxgi::IDXGIOutput,
        Gdi::{GetMonitorInfoW, MONITORINFO, MONITORINFOEXW},
    },
};

use crate::{graphics::RefreshRate, time::FramesPerSecond};

pub struct Dx12Output {
    output: IDXGIOutput,
    refresh_rate: RefreshRate,
}

impl Dx12Output {
    pub fn new(output: IDXGIOutput) -> Self {
        let refresh_rate = get_output_refresh_rate(&output);

        Self {
            output,
            refresh_rate,
        }
    }

    pub fn refresh_rate(&self) -> RefreshRate {
        self.refresh_rate
    }

    pub fn wait_for_vsync(&self) {
        unsafe { self.output.WaitForVBlank() }.unwrap();
    }
}

fn get_output_refresh_rate(output: &IDXGIOutput) -> RefreshRate {
    let monitor = {
        let mut desc = Default::default();
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
        unsafe { GetDisplayConfigBufferSizes(flags, &mut n_paths, &mut n_modes) };

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
            Ok(_) => unsafe {
                paths.set_len(n_paths as usize);
                modes.set_len(n_modes as usize);
                break;
            },
            Err(e) => {
                if tries > 10 {
                    panic!("Failed to query display config (too many retries): {:?}", e);
                }
                if e.code() == ERROR_INSUFFICIENT_BUFFER.into() {
                    tries += 1;
                } else {
                    panic!("Failed to query display config: {:?}", e);
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
