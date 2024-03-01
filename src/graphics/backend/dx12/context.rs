use std::{mem::ManuallyDrop, sync::Arc};

use windows::{
    core::Interface,
    Win32::{
        Foundation::{HANDLE, HWND, RECT},
        Graphics::{
            Direct3D12::{
                ID3D12CommandAllocator, ID3D12DescriptorHeap, ID3D12GraphicsCommandList,
                ID3D12Resource, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
                D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
                D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_VIEWPORT,
            },
            DirectComposition::{IDCompositionDevice, IDCompositionTarget, IDCompositionVisual},
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
    frame::FramesPerSecond,
    geometry::image,
    geometry::window::{DpiScale, WindowSize},
    graphics::{backend::SubmitId, FrameInfo},
    limits::MAX_WINDOW_DIMENSION,
    system::time::Instant,
};

use super::{
    canvas::{Canvas, DrawCommand, DrawList},
    device::Device,
};

pub struct Context {
    device: Arc<Device>,

    swapchain: IDXGISwapChain3,
    #[allow(dead_code)]
    target: IDCompositionTarget,
    #[allow(dead_code)]
    visual: IDCompositionVisual,
    swapchain_ready: HANDLE,
    #[allow(dead_code)]
    swapchain_rtv_heap: ID3D12DescriptorHeap,
    swapchain_rtv: D3D12_CPU_DESCRIPTOR_HANDLE,

    size: WindowSize,
    scale: DpiScale,

    draw_list: DrawList,
    frames_in_flight: [Frame; 2],
    prev_submit: Option<SubmitId>,

    composition_rate: FramesPerSecond,
    target_frame_rate: Option<FramesPerSecond>,

    frame_counter: u64,
    is_visible: bool,
    is_drag_resizing: bool,

    /// A resize event. Deferred until repaint to consolidate graphics work and
    /// in case multiple resize events are received in a single frame.
    deferred_resize: Option<(WindowSize, DpiScale, Option<f32>)>,
}

impl Context {
    #[tracing::instrument(skip(device, compositor))]
    pub fn new(device: Arc<Device>, compositor: &IDCompositionDevice, hwnd: HWND) -> Self {
        let (swapchain, target, visual) = {
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

            let swapchain = device
                .create_swapchain(&swapchain_desc)
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

            unsafe { visual.SetContent(&swapchain) }.unwrap();
            unsafe { compositor.Commit() }.unwrap();

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
                tracing::error!("Failed to create descriptor heap: {:?}", e);
                panic!()
            })
        };

        let swapchain_rtv = unsafe { swapchain_rtv_heap.GetCPUDescriptorHandleForHeapStart() };

        let latency_event = unsafe { swapchain.GetFrameLatencyWaitableObject() };

        let frames_in_flight = [Frame::new(&device), Frame::new(&device)];
        let draw_list = DrawList::new();

        Self {
            device,
            swapchain,
            target,
            visual,
            swapchain_ready: latency_event,
            size: WindowSize::default(),
            scale: DpiScale::default(),
            swapchain_rtv_heap,
            swapchain_rtv,
            draw_list,
            frames_in_flight,
            prev_submit: None,
            frame_counter: 0,
            target_frame_rate: None,
            is_visible: false,
            composition_rate: FramesPerSecond::ZERO,
            is_drag_resizing: false,
            deferred_resize: None,
        }
    }

    pub fn resize(&mut self, size: WindowSize) {
        self.deferred_resize = Some((size, self.scale, None));
    }

    pub fn change_dpi(&mut self, size: WindowSize, scale: DpiScale) {
        self.deferred_resize = Some((size, scale, None));
    }

    pub fn begin_draw(&mut self) -> (Canvas, FrameInfo) {
        // todo: how to handle multiple repaint events in a single frame (when
        // animating and resizing at the same time)? -dz

        unsafe { WaitForSingleObjectEx(self.swapchain_ready, u32::MAX, true) };

        if let Some((size, dpi, flex)) = self.deferred_resize.take() {
            resize_swapchain(&self.swapchain, size, flex, || {
                self.device.wait_for_idle();
            });

            tracing::info!("window: resized to {:?}", size);

            self.size = size;
            self.scale = dpi;
        }

        let canvas = {
            let rect = self.size.into_rect().into();
            Canvas::new(&mut self.draw_list, rect)
        };

        let timings = {
            let prev_present_time = {
                let mut stats = DXGI_FRAME_STATISTICS::default();
                unsafe { self.swapchain.GetFrameStatistics(&mut stats) }
                    .ok()
                    .map_or(Instant::ZERO, |()| {
                        #[allow(clippy::cast_sign_loss)]
                        Instant::from_ticks(stats.SyncQPCTime)
                    })
            };

            let next_present_time = {
                let now = Instant::now();
                let mut time = prev_present_time;
                let frame_time = self.composition_rate.frame_time();

                while time < now {
                    time += frame_time;
                }

                time
            };

            FrameInfo {
                target_frame_rate: self.target_frame_rate,
                prev_present_time,
                next_present_time,
            }
        };

        (canvas, timings)
    }

    pub fn end_draw(&mut self) {
        let image: ID3D12Resource = {
            let index = unsafe { self.swapchain.GetCurrentBackBufferIndex() };
            unsafe { self.swapchain.GetBuffer(index) }.unwrap()
        };

        let frame = &mut self.frames_in_flight[(self.frame_counter % 2) as usize];

        unsafe {
            self.device
                .handle
                .CreateRenderTargetView(&image, None, self.swapchain_rtv);
        };

        self.draw_list.finish();

        let submit_id = upload_draw_list(
            &self.device,
            &self.draw_list,
            frame,
            &image,
            self.swapchain_rtv,
        );

        unsafe { self.swapchain.Present(1, 0) }.unwrap();

        self.frame_counter += 1;
        self.prev_submit = Some(submit_id);

        #[cfg(feature = "profile")]
        tracing_tracy::client::frame_mark();
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // technically, wait for the most recent present to complete
        self.device.wait_for_idle();
    }
}

const DEFAULT_DRAW_BUFFER_SIZE: u64 = 64 * 1024;

pub struct Frame {
    buffer: Option<ID3D12Resource>,
    buffer_size: u64,
    buffer_ptr: *mut u8,

    submit_id: Option<SubmitId>,
    command_list: ID3D12GraphicsCommandList,
    command_list_mem: ID3D12CommandAllocator,
}

impl Frame {
    fn new(device: &Device) -> Self {
        let buffer = device.alloc_buffer(DEFAULT_DRAW_BUFFER_SIZE);

        let buffer_ptr = {
            let mut mapped = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
            mapped.cast()
        };

        let command_allocator: ID3D12CommandAllocator = unsafe {
            device
                .handle
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        }
        .unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.handle.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &command_allocator,
                None,
            )
        }
        .unwrap();

        unsafe { command_list.Close() }.unwrap();

        Self {
            buffer: Some(buffer),
            buffer_size: DEFAULT_DRAW_BUFFER_SIZE,
            buffer_ptr,
            submit_id: None,
            command_list,
            command_list_mem: command_allocator,
        }
    }
}

pub fn image_barrier(
    command_list: &ID3D12GraphicsCommandList,
    image: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
    to: D3D12_RESOURCE_STATES,
) {
    let transition = D3D12_RESOURCE_TRANSITION_BARRIER {
        pResource: unsafe { std::mem::transmute_copy(image) },
        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        StateBefore: from,
        StateAfter: to,
    };

    let barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(transition),
        },
    };

    unsafe { command_list.ResourceBarrier(&[barrier]) };
}

#[tracing::instrument(skip(swapchain, idle))]
pub fn resize_swapchain(
    swapchain: &IDXGISwapChain3,
    size: WindowSize,
    flex: Option<f32>,
    idle: impl Fn(),
) {
    let width = u32::try_from(size.width).unwrap();
    let height = u32::try_from(size.height).unwrap();

    if let Some(flex) = flex {
        let mut desc = DXGI_SWAP_CHAIN_DESC1::default();
        unsafe { swapchain.GetDesc1(&mut desc) }.unwrap();

        if width > desc.Width || height > desc.Height {
            #[allow(clippy::cast_sign_loss)]
            let w = ((width as f32) * flex).min(f32::from(MAX_WINDOW_DIMENSION)) as u32;
            #[allow(clippy::cast_sign_loss)]
            let h = ((height as f32) * flex).min(f32::from(MAX_WINDOW_DIMENSION)) as u32;

            idle();
            unsafe {
                swapchain.ResizeBuffers(
                    0,
                    w,
                    h,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
                )
            }
            .unwrap();
        }

        unsafe { swapchain.SetSourceSize(width, height) }.unwrap();
    } else {
        idle();
        unsafe {
            swapchain.ResizeBuffers(
                0,
                width,
                height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as _,
            )
        }
        .unwrap();
    }
}

#[allow(clippy::too_many_lines)]
fn upload_draw_list(
    device: &Device,
    content: &DrawList,
    frame: &mut Frame,
    target: &ID3D12Resource,
    target_rtv: D3D12_CPU_DESCRIPTOR_HANDLE,
) -> SubmitId {
    if let Some(submit_id) = frame.submit_id {
        device.wait(submit_id);
    }

    {
        #[cfg(feature = "profile")]
        let _s = tracing_tracy::client::span!("copy rects to buffer");

        let rects_size = std::mem::size_of_val(content.rects.as_slice());
        let buffer_size = rects_size as u64;

        if frame.buffer_size < buffer_size {
            let _ = frame.buffer.take();

            let buffer = device.alloc_buffer(buffer_size);

            frame.buffer_ptr = {
                let mut mapped = std::ptr::null_mut();
                unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
                mapped.cast()
            };

            frame.buffer = Some(buffer);
            frame.buffer_size = buffer_size;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                content.rects.as_ptr().cast(),
                frame.buffer_ptr,
                rects_size,
            );
        }
    }

    let viewpowrt: image::Box = {
        assert!(content.commands[0].0 == DrawCommand::Begin);
        content.areas[0].into()
    };

    let viewport_scale = [
        1.0 / f32::from(viewpowrt.right),
        1.0 / f32::from(viewpowrt.bottom),
    ];

    let mut rect_idx = 0;
    // let mut area_idx = 0;
    let mut color_idx = 0;

    for (command, idx) in &content.commands {
        match command {
            DrawCommand::Begin => unsafe {
                frame.command_list_mem.Reset().unwrap();
                frame
                    .command_list
                    .Reset(&frame.command_list_mem, None)
                    .unwrap();

                image_barrier(
                    &frame.command_list,
                    target,
                    D3D12_RESOURCE_STATE_PRESENT,
                    D3D12_RESOURCE_STATE_RENDER_TARGET,
                );

                frame
                    .command_list
                    .OMSetRenderTargets(1, Some(&target_rtv), false, None);

                frame.command_list.RSSetViewports(&[D3D12_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: f32::from(viewpowrt.right),
                    Height: f32::from(viewpowrt.bottom),
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                }]);

                frame.command_list.RSSetScissorRects(&[RECT {
                    left: i32::from(viewpowrt.left),
                    top: i32::from(viewpowrt.top),
                    right: i32::from(viewpowrt.right),
                    bottom: i32::from(viewpowrt.bottom),
                }]);
            },
            DrawCommand::End => unsafe {
                image_barrier(
                    &frame.command_list,
                    target,
                    D3D12_RESOURCE_STATE_RENDER_TARGET,
                    D3D12_RESOURCE_STATE_PRESENT,
                );
                frame.command_list.Close().unwrap();
            },
            DrawCommand::Clear => {
                unsafe {
                    frame.command_list.ClearRenderTargetView(
                        target_rtv,
                        &content.clears[color_idx].to_array_f32(),
                        None,
                    );
                }
                color_idx += 1;
            }
            DrawCommand::DrawRects => {
                let num_rects = *idx - rect_idx;

                device.rect_shader.bind(
                    &frame.command_list,
                    frame.buffer.as_ref().unwrap(),
                    viewport_scale,
                    (viewpowrt.bottom.checked_sub(viewpowrt.top)).unwrap() as f32,
                );

                unsafe { frame.command_list.DrawInstanced(4, num_rects, 0, rect_idx) };

                rect_idx = *idx;
            }
        }
    }

    let submit_id = device.submit(&frame.command_list);

    frame.submit_id = Some(submit_id);

    submit_id
}
