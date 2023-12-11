use windows::{
    core::{w, ComInterface},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D12::ID3D12Resource,
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain3, DXGI_PRESENT_RESTART, DXGI_RGBA, DXGI_SCALING_NONE,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
    },
};

use crate::graphics::{backend::dx12::Dx12Image, Image, ResizeOp, SubmissionId};

use super::Dx12Device;

pub struct Dx12Swapchain {
    handle: IDXGISwapChain3,
    images: Option<[Image; 2]>,
    was_resized: bool,
    last_present: Option<SubmissionId>,
}

impl Dx12Swapchain {
    pub fn new(device: &Dx12Device, window: HWND) -> Self {
        let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: 0,  // extract from hwnd
            Height: 0, // extract from hwnd
            Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,   // required by FLIP_SEQUENTIAL
                Quality: 0, // required by FLIP_SEQUENTIAL
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE, // backbuffer tranparency is ignored
            Flags: 0,
        };

        let swapchain = unsafe {
            device.dxgi_factory.CreateSwapChainForHwnd(
                &device.graphics_queue.queue,
                window,
                &swapchain_desc,
                None,
                None,
            )
        }
        .unwrap_or_else(|e| {
            tracing::error!("Failed to create swapchain: {:?}", e);
            panic!()
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
            swapchain
                .SetBackgroundColor(&DXGI_RGBA {
                    r: 0.0,
                    g: 0.2,
                    b: 0.4,
                    a: 1.0,
                })
                .unwrap();
        }

        let images = Self::get_images(&swapchain, device);
        Self {
            handle: swapchain,
            images: Some(images),
            was_resized: false,
            last_present: None,
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
            ResizeOp::Auto => resize_swapchain(device, self, 0, 0),
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
        unsafe { self.handle.GetContainingOutput().unwrap().WaitForVBlank() }.unwrap();
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
