use std::cell::RefCell;

use windows::{
    core::ComInterface,
    Win32::{
        Foundation::{HANDLE, HWND, RECT},
        Graphics::{
            Direct3D12::{
                ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_VIEWPORT,
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
            Gdi::{RedrawWindow, RDW_INTERNALPAINT},
        },
        System::Threading::WaitForSingleObjectEx,
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::{
    frame::{FrameId, FramesPerSecond, RedrawRequest},
    graphics::{Canvas, FrameInfo},
    limits::MAX_WINDOW_DIMENSION,
    math::{Point, Rect, Scale, Size},
    platform::{
        gfx::{DrawCommand, DrawList},
        AppContextImpl, GraphicsInterposer, VSyncRequest,
    },
    time::Instant,
    Axis, ButtonState, EventHandler, LogicalPixel, MouseButton, PhysicalPixel,
};

use super::{descriptor::SingleDescriptorHeap, image_barrier, queue::SubmitId, Device, Frame};

pub struct DxWindow<W: EventHandler> {
    inner: RefCell<DxWindowImpl<W>>,
}

impl<W: EventHandler> DxWindow<W> {
    pub fn new(context: AppContextImpl, user_handler: W, hwnd: HWND) -> Self {
        let inner = DxWindowImpl::new(context, user_handler, hwnd);
        Self {
            inner: RefCell::new(inner),
        }
    }
}

impl<W: EventHandler> GraphicsInterposer for DxWindow<W> {
    #[inline]
    fn on_close_request(&self) {
        self.inner.borrow_mut().on_close_request();
    }

    #[inline]
    fn on_visible(&self, visible: bool) {
        self.inner.borrow_mut().on_visible(visible);
    }

    #[inline]
    fn on_begin_resize(&self) {
        self.inner.borrow_mut().on_begin_resize();
    }

    #[inline]
    fn on_resize(
        &self,
        size: Size<u16, PhysicalPixel>,
        scale: Scale<f32, PhysicalPixel, LogicalPixel>,
    ) {
        self.inner.borrow_mut().on_resize(size, scale);
    }

    #[inline]
    fn on_end_resize(&self) {
        self.inner.borrow_mut().on_end_resize();
    }

    #[inline]
    fn on_mouse_button(
        &self,
        button: MouseButton,
        state: ButtonState,
        location: Point<i16, PhysicalPixel>,
    ) {
        self.inner
            .borrow_mut()
            .on_mouse_button(button, state, location);
    }

    #[inline]
    fn on_pointer_move(&self, location: Point<i16, PhysicalPixel>) {
        self.inner.borrow_mut().on_pointer_move(location);
    }

    #[inline]
    fn on_pointer_leave(&self) {
        self.inner.borrow_mut().on_pointer_leave();
    }

    #[inline]
    fn on_scroll(&self, axis: Axis, delta: f32) {
        self.inner.borrow_mut().on_scroll(axis, delta);
    }

    #[inline]
    fn on_os_paint(&self) {
        self.inner.borrow_mut().on_os_paint();
    }

    #[inline]
    fn on_vsync(&self, frame_id: FrameId, rate: Option<FramesPerSecond>) {
        self.inner.borrow_mut().on_vsync(frame_id, rate);
    }

    #[inline]
    fn on_composition_rate(&self, frame_id: FrameId, rate: FramesPerSecond) {
        self.inner.borrow_mut().on_composition_rate(frame_id, rate);
    }

    #[inline]
    fn on_redraw_request(&self, request: RedrawRequest) {
        self.inner.borrow_mut().on_redraw_request(request);
    }
}

pub struct DxWindowImpl<W: EventHandler> {
    user_handler: W,

    hwnd: HWND,
    app: AppContextImpl,
    device: Device,

    swapchain: IDXGISwapChain3,
    #[allow(dead_code)]
    target: IDCompositionTarget,
    #[allow(dead_code)]
    visual: IDCompositionVisual,
    swapchain_ready: HANDLE,
    swapchain_rtv: SingleDescriptorHeap,

    size: Size<u16, PhysicalPixel>,

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
    deferred_resize: Option<(Size<u16, PhysicalPixel>, Option<f32>)>,
}

impl<W: EventHandler> DxWindowImpl<W> {
    #[tracing::instrument(skip(app, user_handler))]
    fn new(app: AppContextImpl, user_handler: W, hwnd: HWND) -> Self {
        let (device, swapchain, target, visual) = {
            let app = &app.inner.read(); // needs to be here to drop the lock before creating `Self`.
            let device = app.dx12.clone();

            let target = unsafe { app.compositor.CreateTargetForHwnd(hwnd, true) }.unwrap();
            let visual = unsafe { app.compositor.CreateVisual() }.unwrap();
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

            let swapchain = unsafe {
                app.dxgi
                    .CreateSwapChainForComposition(device.queue(), &swapchain_desc, None)
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
            unsafe { app.compositor.Commit() }.unwrap();

            (device, swapchain, target, visual)
        };

        let swapchain_rtv = SingleDescriptorHeap::new(&device, D3D12_DESCRIPTOR_HEAP_TYPE_RTV);

        let latency_event = unsafe { swapchain.GetFrameLatencyWaitableObject() };

        let frames_in_flight = [Frame::new(&device), Frame::new(&device)];
        let draw_list = DrawList::new();

        Self {
            hwnd,
            user_handler,
            app,
            swapchain,
            target,
            visual,
            swapchain_ready: latency_event,
            size: Size::default(),
            device,
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

    #[tracing::instrument(skip(self))]
    fn on_close_request(&mut self) {
        self.user_handler.on_close_request();
    }

    #[tracing::instrument(skip(self))]
    fn on_visible(&mut self, visible: bool) {
        self.is_visible = visible;
        self.user_handler.on_visible(visible);
    }

    #[tracing::instrument(skip(self))]
    fn on_begin_resize(&mut self) {
        self.is_drag_resizing = true;
        self.user_handler.on_begin_resize();
    }

    #[tracing::instrument(skip(self))]
    fn on_resize(
        &mut self,
        size: Size<u16, PhysicalPixel>,
        scale: Scale<f32, PhysicalPixel, LogicalPixel>,
    ) {
        self.deferred_resize = Some((size, None));
        self.user_handler.on_resize(size, scale);
    }

    #[tracing::instrument(skip(self))]
    fn on_end_resize(&mut self) {
        self.is_drag_resizing = false;
        self.user_handler.on_end_resize();
    }

    #[tracing::instrument(skip(self))]
    fn on_mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        location: Point<i16, PhysicalPixel>,
    ) {
        self.user_handler.on_mouse_button(button, state, location);
    }

    #[tracing::instrument(skip(self))]
    fn on_pointer_move(&mut self, location: Point<i16, PhysicalPixel>) {
        self.user_handler.on_pointer_move(location);
    }

    #[tracing::instrument(skip(self))]
    fn on_pointer_leave(&mut self) {
        self.user_handler.on_pointer_leave();
    }

    #[tracing::instrument(skip(self))]
    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        self.user_handler.on_scroll(axis, delta);
    }

    #[tracing::instrument(skip(self))]
    fn on_os_paint(&mut self) {
        unsafe { WaitForSingleObjectEx(self.swapchain_ready, u32::MAX, true) };

        if let Some((size, flex)) = self.deferred_resize.take() {
            resize_swapchain(&self.swapchain, size.width, size.height, flex, || {
                self.device.wait_for_idle();
            });

            tracing::info!("window: resized to {:?}", size);

            self.size = size;
        }

        let mut canvas = {
            let rect = {
                let size = self.size;
                Rect::new(0, 0, size.width, size.height)
            };

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

        self.user_handler.on_repaint(&mut canvas, &timings);

        let image: ID3D12Resource = {
            let index = unsafe { self.swapchain.GetCurrentBackBufferIndex() };
            unsafe { self.swapchain.GetBuffer(index) }.unwrap()
        };

        let frame = &mut self.frames_in_flight[(self.frame_counter % 2) as usize];

        let rtv = {
            let rtv = self.swapchain_rtv.cpu_handle();

            unsafe {
                self.device
                    .inner
                    .device
                    .CreateRenderTargetView(&image, None, rtv);
            }

            rtv
        };

        let submit_id = upload_draw_list(&self.device, canvas.finish(), frame, &image, rtv);

        unsafe { self.swapchain.Present(1, 0) }.unwrap();

        self.frame_counter += 1;
        self.prev_submit = Some(submit_id);

        #[cfg(feature = "profile")]
        tracing_tracy::client::frame_mark();
    }

    #[tracing::instrument(skip(self))]
    fn on_vsync(&mut self, _frame_id: FrameId, rate: Option<FramesPerSecond>) {
        self.target_frame_rate = rate;
    }

    #[tracing::instrument(skip(self))]
    fn on_composition_rate(&mut self, _frame_id: FrameId, rate: FramesPerSecond) {
        self.composition_rate = rate;
    }

    #[tracing::instrument(skip(self))]
    fn on_redraw_request(&mut self, request: RedrawRequest) {
        let send = |r| self.app.vsync_sender.send(r).unwrap();

        match request {
            RedrawRequest::Idle => send(VSyncRequest::Idle(self.hwnd)),
            RedrawRequest::Once => unsafe {
                RedrawWindow(self.hwnd, None, None, RDW_INTERNALPAINT);
            },
            RedrawRequest::AtFrame(frame_id) => send(VSyncRequest::AtFrame(self.hwnd, frame_id)),
            RedrawRequest::AtFrameRate(rate) => send(VSyncRequest::AtFrameRate(self.hwnd, rate)),
        }
    }
}

#[tracing::instrument(skip(swapchain, idle))]
pub fn resize_swapchain(
    swapchain: &IDXGISwapChain3,
    width: u16,
    height: u16,
    flex: Option<f32>,
    idle: impl Fn(),
) {
    if let Some(flex) = flex {
        let mut desc = DXGI_SWAP_CHAIN_DESC1::default();
        unsafe { swapchain.GetDesc1(&mut desc) }.unwrap();

        if u32::from(width) > desc.Width || u32::from(height) > desc.Height {
            #[allow(clippy::cast_sign_loss)]
            let w = ((f32::from(width)) * flex).min(f32::from(MAX_WINDOW_DIMENSION)) as u32;
            #[allow(clippy::cast_sign_loss)]
            let h = ((f32::from(height)) * flex).min(f32::from(MAX_WINDOW_DIMENSION)) as u32;

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

        unsafe { swapchain.SetSourceSize(u32::from(width), u32::from(height)) }.unwrap();
    } else {
        idle();
        unsafe {
            swapchain.ResizeBuffers(
                0,
                u32::from(width),
                u32::from(height),
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

    let viewport_rect = {
        assert!(content.commands[0].0 == DrawCommand::Begin);
        content.areas[0]
    };

    let viewport_scale = [
        1.0 / f32::from(viewport_rect.right()),
        1.0 / f32::from(viewport_rect.bottom()),
    ];

    let mut rect_idx = 0;
    let mut area_idx = 0;
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
                    Width: viewport_rect.right().into(),
                    Height: viewport_rect.bottom().into(),
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                }]);

                frame.command_list.RSSetScissorRects(&[RECT {
                    left: i32::from(viewport_rect.left()),
                    top: i32::from(viewport_rect.top()),
                    right: i32::from(viewport_rect.right()),
                    bottom: i32::from(viewport_rect.bottom()),
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

                device.inner.rect_shader.bind(
                    &frame.command_list,
                    frame.buffer.as_ref().unwrap(),
                    viewport_scale,
                    viewport_rect.height.into(),
                );

                unsafe { frame.command_list.DrawInstanced(4, num_rects, 0, rect_idx) };

                rect_idx = *idx;
            }
        }
    }

    let submit_id = device.submit_graphics(&frame.command_list);

    frame.submit_id = Some(submit_id);

    submit_id
}
