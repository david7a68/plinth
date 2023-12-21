use std::{sync::OnceLock, thread::current};

use windows::{
    core::{w, ComInterface},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D12::ID3D12Resource,
            DirectComposition::{
                DCompositionWaitForCompositorClock, IDCompositionDevice, IDCompositionTarget,
                IDCompositionVisual, DCOMPOSITION_FRAME_STATISTICS,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain3, DXGI_PRESENT_RESTART, DXGI_RGBA, DXGI_SCALING_STRETCH,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::graphics::{
    backend::dx12::Dx12Image, Image, PresentInstant, PresentStatistics, ResizeOp, SubmissionId,
};

use super::Dx12Device;

static CAN_WAIT_FOR_COMPOSITOR_CLOCK: OnceLock<bool> = OnceLock::new();

pub struct Dx12Swapchain {
    handle: IDXGISwapChain3,

    #[allow(dead_code)]
    target: IDCompositionTarget,

    #[allow(dead_code)]
    visual: IDCompositionVisual,

    compositor: IDCompositionDevice,

    images: Option<[Image; 2]>,
    was_resized: bool,
    last_present: Option<SubmissionId>,
}

impl Dx12Swapchain {
    pub fn new(device: &Dx12Device, window: HWND) -> Self {
        CAN_WAIT_FOR_COMPOSITOR_CLOCK.get_or_init(|| {
            let version = windows_version::OsVersion::current();
            version.major >= 10 && version.build >= 22000
        });

        let target = unsafe { device.compositor.CreateTargetForHwnd(window, true) }.unwrap();
        let visual = unsafe { device.compositor.CreateVisual() }.unwrap();
        unsafe { target.SetRoot(&visual) }.unwrap();

        let (width, height) = {
            let mut rect = Default::default();
            unsafe { GetClientRect(window, &mut rect) }.unwrap();
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
            Flags: 0,
        };

        let handle = unsafe {
            device.dxgi_factory.CreateSwapChainForComposition(
                &device.graphics_queue.queue,
                &swapchain_desc,
                None,
            )
        }
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
        unsafe { device.compositor.Commit() }.unwrap();

        let images = Self::get_images(&handle, device);

        Self {
            target,
            visual,
            handle,
            compositor: device.compositor.clone(),
            images: Some(images),
            was_resized: false,
            last_present: None,
        }
    }

    pub fn present_statistics(&self) -> PresentStatistics {
        let mut statistics = DCOMPOSITION_FRAME_STATISTICS::default();
        unsafe { self.compositor.GetFrameStatistics(&mut statistics) }.unwrap();

        let num = statistics.currentCompositionRate.Numerator as f64;
        let den = statistics.currentCompositionRate.Denominator as f64;
        let current_rate = num / den;

        let prev_present_time = PresentInstant::from_ticks(statistics.lastFrameTime as u64);
        let next_estimated_present_time =
            PresentInstant::from_ticks(statistics.nextEstimatedFrameTime as u64);

        PresentStatistics {
            monitor_rate: current_rate as f32,
            prev_present_time,
            next_estimated_present_time,
        }
    }

    pub fn resize(&mut self, device: &Dx12Device, op: ResizeOp) {
        fn resize_swapchain(
            device: &Dx12Device,
            swapchain: &mut Dx12Swapchain,
            width: u32,
            height: u32,
        ) {
            device.wait_for_idle();

            if let Some(last_present) = swapchain.last_present {
                device.wait_until(last_present);
            }

            {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("free swapchain images");

                let images = swapchain.images.take().unwrap();
                let image0: &Dx12Image = (&images[0]).try_into().unwrap();
                let image1: &Dx12Image = (&images[1]).try_into().unwrap();

                {
                    let mut rt = device.render_target_descriptor_heap.lock();
                    rt.deallocate(image0.render_target_view);
                    rt.deallocate(image1.render_target_view);
                }

                std::mem::drop(images);
            }

            unsafe {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("resize buffers");

                swapchain
                    .handle
                    .ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, 0)
            }
            .unwrap();

            swapchain.images = Some(Dx12Swapchain::get_images(&swapchain.handle, device));
        }

        match op {
            ResizeOp::Fixed { width, height } => resize_swapchain(device, self, width, height),
            ResizeOp::Flex {
                width,
                height,
                flex,
            } => {
                let mut desc = Default::default();
                unsafe { self.handle.GetDesc1(&mut desc) }.unwrap();

                if width > desc.Width || height > desc.Height {
                    let w = ((width as f32) * flex).min(u16::MAX as f32) as u32;
                    let h = ((height as f32) * flex).min(u16::MAX as f32) as u32;

                    resize_swapchain(device, self, w, h);
                }

                unsafe { self.handle.SetSourceSize(width, height) }.unwrap();
            }
        }

        self.was_resized = true;
    }

    pub fn get_back_buffer(&self) -> (&Image, u32) {
        let index = unsafe { self.handle.GetCurrentBackBufferIndex() };
        let image = &(self.images.as_ref().unwrap())[index as usize];
        (image, index)
    }

    pub fn present(&mut self, submission_id: SubmissionId) {
        let flags = if self.was_resized {
            self.was_resized = false;
            DXGI_PRESENT_RESTART
        } else {
            0
        };

        unsafe { self.handle.Present(1, flags) }.unwrap();
        self.last_present = Some(submission_id);
    }

    pub fn wait_for_vsync(&self) {
        if *CAN_WAIT_FOR_COMPOSITOR_CLOCK.get().unwrap() {
            unsafe { DCompositionWaitForCompositorClock(None, u32::MAX) };
        } else {
            todo!("wait for vsync without DCompositionWaitForCompositorClock")
        }
    }

    #[tracing::instrument(skip(swapchain, device))]
    fn get_images(swapchain: &IDXGISwapChain3, device: &Dx12Device) -> [Image; 2] {
        let image0: ID3D12Resource = unsafe { swapchain.GetBuffer(0) }.unwrap();
        let image1: ID3D12Resource = unsafe { swapchain.GetBuffer(1) }.unwrap();

        let (view0, view1) = {
            let mut rt = device.render_target_descriptor_heap.lock();
            (rt.allocate().unwrap(), rt.allocate().unwrap())
        };

        unsafe {
            image0.SetName(w!("Backbuffer 0")).unwrap();
            image1.SetName(w!("Backbuffer 1")).unwrap();

            device.device.CreateRenderTargetView(&image0, None, view0);
            device.device.CreateRenderTargetView(&image1, None, view1);
        }

        [
            Dx12Image {
                handle: image0,
                render_target_view: view0,
            }
            .into(),
            Dx12Image {
                handle: image1,
                render_target_view: view1,
            }
            .into(),
        ]
    }
}

unsafe impl Send for Dx12Swapchain {}
