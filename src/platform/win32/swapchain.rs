use windows::{
    core::ComInterface,
    Win32::{
        Foundation::{HANDLE, HWND, RECT},
        Graphics::{
            Direct3D12::{ID3D12CommandQueue, ID3D12Resource},
            DirectComposition::{IDCompositionDevice, IDCompositionTarget, IDCompositionVisual},
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                IDXGIFactory2, IDXGISwapChain3, DXGI_FRAME_STATISTICS, DXGI_RGBA,
                DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
                DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::WaitForSingleObjectEx,
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::time::Instant;

pub struct Swapchain {
    handle: IDXGISwapChain3,

    #[allow(dead_code)]
    target: IDCompositionTarget,

    #[allow(dead_code)]
    visual: IDCompositionVisual,

    event: HANDLE,

    size: (u16, u16),
}

impl Swapchain {
    pub fn new(
        dxgi: &IDXGIFactory2,
        compositor: &IDCompositionDevice,
        queue: &ID3D12CommandQueue,
        hwnd: HWND,
    ) -> Self {
        let target = unsafe { compositor.CreateTargetForHwnd(hwnd, true) }.unwrap();
        let visual = unsafe { compositor.CreateVisual() }.unwrap();
        unsafe { target.SetRoot(&visual) }.unwrap();

        let (width, height) = {
            let mut rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
            (rect.right - rect.left, rect.bottom - rect.top)
        };

        let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width as u32,   // extract from hwnd
            Height: height as u32, // extract from hwnd
            Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,   // required by FLIP_SEQUENTIAL
                Quality: 0, // required by FLIP_SEQUENTIAL
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE, // backbuffer tranparency is ignored
            Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
        };

        let handle = unsafe { dxgi.CreateSwapChainForComposition(queue, &swapchain_desc, None) }
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create swapchain: {:?}", e);
                panic!();
            })
            .cast::<IDXGISwapChain3>()
            .unwrap_or_else(|e| {
                tracing::error!(
                    "The running version of windows doesn't support IDXGISwapchain3. Error: {:?}",
                    e
                );
                panic!()
            });

        unsafe {
            handle
                .SetBackgroundColor(&DXGI_RGBA {
                    r: 0.0,
                    g: 0.2,
                    b: 0.4,
                    a: 1.0,
                })
                .unwrap();
        }

        unsafe { visual.SetContent(&handle) }.unwrap();
        unsafe { compositor.Commit() }.unwrap();

        let event = unsafe { handle.GetFrameLatencyWaitableObject() };

        Self {
            handle,
            target,
            visual,
            event,
            size: (width as u16, height as u16),
        }
    }

    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    pub fn prev_present_time(&self) -> Option<Instant> {
        let mut stats = DXGI_FRAME_STATISTICS::default();

        unsafe { self.handle.GetFrameStatistics(&mut stats) }
            .ok()
            .map(|()| Instant::from_ticks(stats.SyncQPCTime as u64))
    }

    #[tracing::instrument(skip(self, idle))]
    pub fn resize(&mut self, width: u16, height: u16, flex: Option<f32>, idle: impl Fn()) {
        let width = width as u32;
        let height = height as u32;

        if let Some(flex) = flex {
            let mut desc = Default::default();
            unsafe { self.handle.GetDesc1(&mut desc) }.unwrap();

            if width > desc.Width || height > desc.Height {
                let w = ((width as f32) * flex).min(u16::MAX as f32) as u32;
                let h = ((height as f32) * flex).min(u16::MAX as f32) as u32;

                idle();
                unsafe {
                    self.handle.ResizeBuffers(
                        0,
                        w,
                        h,
                        DXGI_FORMAT_UNKNOWN,
                        DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
                    )
                }
                .unwrap();
            }

            unsafe { self.handle.SetSourceSize(width, height) }.unwrap();
        } else {
            idle();
            unsafe {
                self.handle.ResizeBuffers(
                    0,
                    width,
                    height,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
                )
            }
            .unwrap();
        }

        self.size = (width as u16, height as u16);
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_for_present(&self) {
        unsafe { WaitForSingleObjectEx(self.event, u32::MAX, true) };
    }

    #[tracing::instrument(skip(self))]
    pub fn get_back_buffer(&self) -> (ID3D12Resource, u32) {
        let index = unsafe { self.handle.GetCurrentBackBufferIndex() };
        let image = unsafe { self.handle.GetBuffer(index) }.unwrap();
        (image, index)
    }

    #[tracing::instrument(skip(self))]
    pub fn present(&mut self) {
        unsafe { self.handle.Present(1, 0) }.unwrap();
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // self.handle.getrst
    }
}
