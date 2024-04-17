use windows::{
    core::Interface,
    Win32::{
        Foundation::{HANDLE, HWND, RECT},
        Graphics::{
            Direct3D12::{
                ID3D12DescriptorHeap, ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_RESOURCE_STATE_PRESENT,
            },
            DirectComposition::{IDCompositionTarget, IDCompositionVisual},
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain3, DXGI_FRAME_STATISTICS, DXGI_RGBA, DXGI_SCALING_STRETCH,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
                DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::WaitForSingleObjectEx,
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::{
    core::limit::Limit,
    graphics::{FrameInfo, RenderTarget as BRenderTarget},
    system::WindowExtent,
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use super::{device::Device, RenderTarget};

pub struct SwapchainImage<'a, 'b> {
    swapchain: &'a mut Swapchain<'b>,
    render_target: BRenderTarget,
    timings: FrameInfo,
}

impl SwapchainImage<'_, '_> {
    pub fn render_target(&self) -> &BRenderTarget {
        &self.render_target
    }

    pub fn render_target_mut(&mut self) -> &mut BRenderTarget {
        &mut self.render_target
    }

    pub fn frame_info(&self) -> FrameInfo {
        self.timings
    }

    pub fn present(self) {
        let BRenderTarget::Dx12(render_target) = self.render_target else {
            unreachable!();
        };

        let Some(_) = render_target.draw else {
            panic!("No draw submitted to the swapchain image. Cannot present.");
        };

        self.swapchain.present();
    }
}

pub struct Swapchain<'a> {
    device: &'a Device,

    handle: IDXGISwapChain3,
    #[allow(dead_code)]
    target: IDCompositionTarget,
    #[allow(dead_code)]
    visual: IDCompositionVisual,
    waitable_object: HANDLE,

    #[allow(dead_code)]
    rtv_heap: ID3D12DescriptorHeap,
    rtv: D3D12_CPU_DESCRIPTOR_HANDLE,

    size: WindowExtent,

    composition_rate: FramesPerSecond,
    target_frame_rate: Option<FramesPerSecond>,

    is_visible: bool,
    is_drag_resizing: bool,
}

impl<'device> Swapchain<'device> {
    pub(super) fn new(device: &'device Device, hwnd: HWND) -> Self {
        let (swapchain, target, visual) = {
            let target = unsafe { device.compositor.CreateTargetForHwnd(hwnd, true) }.unwrap();
            let visual = unsafe { device.compositor.CreateVisual() }.unwrap();
            unsafe { target.SetRoot(&visual) }.unwrap();

            let (width, height) = {
                let mut rect = RECT::default();
                unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
                (rect.right - rect.left, rect.bottom - rect.top)
            };

            let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: u32::try_from(width).unwrap(),   // extract from hwnd
                Height: u32::try_from(height).unwrap(), // extract from hwnd
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

            let swapchain = unsafe {
                device.dxgi.CreateSwapChainForComposition(
                    &device.queue.handle,
                    &swapchain_desc,
                    None,
                )
            }
            .unwrap_or_else(|e| {
                eprintln!("Failed to create swapchain: {e:?}");
                panic!();
            })
            .cast::<IDXGISwapChain3>()
            .unwrap_or_else(|e| {
                eprintln!(
                    "The running version of windows doesn't support IDXGISwapchain3. Error: {e:?}",
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

            unsafe { visual.SetContent(&swapchain) }.unwrap();
            unsafe { device.compositor.Commit() }.unwrap();

            (swapchain, target, visual)
        };

        let swapchain_rtv_heap: ID3D12DescriptorHeap = {
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: 1,
                Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask: 0,
            };

            unsafe { device.handle.CreateDescriptorHeap(&heap_desc) }.unwrap_or_else(|e| {
                eprintln!("Failed to create descriptor heap: {e:?}");
                panic!()
            })
        };

        let swapchain_rtv = unsafe { swapchain_rtv_heap.GetCPUDescriptorHandleForHeapStart() };

        let latency_event = unsafe { swapchain.GetFrameLatencyWaitableObject() };

        Self {
            device,
            handle: swapchain,
            target,
            visual,
            waitable_object: latency_event,
            size: WindowExtent::ZERO,
            rtv_heap: swapchain_rtv_heap,
            rtv: swapchain_rtv,
            target_frame_rate: None,
            is_visible: false,
            composition_rate: FramesPerSecond::default(),
            is_drag_resizing: false,
        }
    }

    pub fn resize(&mut self, size: WindowExtent) {
        resize_swapchain(&self.handle, size, None, || {
            self.device.idle();
        });

        self.size = size;
    }

    pub fn frame_info(&self) -> FrameInfo {
        let prev_present_time = {
            let mut stats = DXGI_FRAME_STATISTICS::default();
            unsafe { self.handle.GetFrameStatistics(&mut stats) }
                .ok()
                .map_or(PresentTime::default(), |()| {
                    #[allow(clippy::cast_sign_loss)]
                    PresentTime::from_qpc_time(stats.SyncQPCTime)
                })
        };

        let next_present_time = {
            let now = PresentTime::now();
            let mut time = prev_present_time;
            let frame_time = self.composition_rate.into();

            while time < now {
                time += frame_time;
            }

            time
        };

        FrameInfo {
            target_frame_rate: self.target_frame_rate,
            vblank_period: PresentPeriod::default(),
            next_present_time,
            prev_present_time,
            prev_target_present_time: PresentTime::default(),
        }
    }

    pub fn next_image<'this>(&'this mut self) -> SwapchainImage<'this, 'device> {
        unsafe { WaitForSingleObjectEx(self.waitable_object, u32::MAX, true) };

        let image: ID3D12Resource = {
            let index = unsafe { self.handle.GetCurrentBackBufferIndex() };
            unsafe { self.handle.GetBuffer(index) }.unwrap()
        };

        unsafe {
            self.device
                .handle
                .CreateRenderTargetView(&image, None, self.rtv);
        };

        let rt = RenderTarget {
            draw: None,
            size: self.size.into(),
            state: D3D12_RESOURCE_STATE_PRESENT,
            resource: image,
            descriptor: self.rtv,
        };

        let fi = self.frame_info();

        SwapchainImage {
            swapchain: self,
            render_target: BRenderTarget::Dx12(rt),
            timings: fi,
        }
    }

    fn present(&mut self) {
        unsafe { self.handle.Present(1, 0) }.unwrap();
    }
}

impl Drop for Swapchain<'_> {
    fn drop(&mut self) {
        // technically, wait for the most recent present to complete
        self.device.idle();
    }
}

pub fn resize_swapchain(
    swapchain: &IDXGISwapChain3,
    size: WindowExtent,
    flex: Option<f32>,
    idle: impl Fn(),
) {
    let width = u32::try_from(size.width).unwrap();
    let height = u32::try_from(size.height).unwrap();

    let resize = |width, height| unsafe {
        swapchain
            .ResizeBuffers(
                0,
                width,
                height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
            )
            .unwrap();
    };

    if let Some(flex) = flex {
        let mut desc = DXGI_SWAP_CHAIN_DESC1::default();
        unsafe { swapchain.GetDesc1(&mut desc) }.unwrap();

        if width > desc.Width || height > desc.Height {
            let max_dim = WindowExtent::max();
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let w = (f64::from(width) * f64::from(flex)).min(f64::from(max_dim.width)) as u32;
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let h = (f64::from(height) * f64::from(flex)).min(f64::from(max_dim.height)) as u32;

            idle();
            resize(w, h);
        }

        unsafe { swapchain.SetSourceSize(width, height) }.unwrap();
    } else {
        idle();
        resize(width, height);
    }
}
