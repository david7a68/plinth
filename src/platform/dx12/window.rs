use std::cell::RefCell;

use windows::{
    core::ComInterface,
    Win32::{
        Foundation::{HANDLE, HWND, RECT},
        Graphics::{
            Direct3D12::ID3D12Resource,
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
    math::Rect,
    platform::{
        dx12::Context,
        gfx::{Context as _, Device, DrawList, SubmitId},
        AppContextImpl, VSyncRequest, Win32WindowEventInterposer,
    },
    time::Instant,
    Input, WindowEvent, WindowEventHandler, WindowSize,
};

use super::Frame;

pub struct DxWindow<W: WindowEventHandler> {
    inner: RefCell<DxWindowImpl<W>>,
}

impl<W: WindowEventHandler> DxWindow<W> {
    pub fn new(context: AppContextImpl, user_handler: W, hwnd: HWND) -> Self {
        let inner = DxWindowImpl::new(context, user_handler, hwnd);
        Self {
            inner: RefCell::new(inner),
        }
    }
}

impl<W: WindowEventHandler> Win32WindowEventInterposer for DxWindow<W> {
    fn on_event(&self, event: WindowEvent) {
        self.inner.borrow_mut().on_event(event);
    }

    fn on_input(&self, input: Input) {
        self.inner.borrow_mut().on_input(input);
    }

    fn on_os_paint(&self) {
        self.inner.borrow_mut().on_os_paint();
    }

    fn on_vsync(&self, frame_id: FrameId, rate: Option<FramesPerSecond>) {
        self.inner.borrow_mut().on_vsync(frame_id, rate);
    }

    fn on_composition_rate(&self, frame_id: FrameId, rate: FramesPerSecond) {
        self.inner.borrow_mut().on_composition_rate(frame_id, rate);
    }

    fn on_redraw_request(&self, request: RedrawRequest) {
        self.inner.borrow_mut().on_redraw_request(request);
    }
}

pub struct DxWindowImpl<W: WindowEventHandler> {
    user_handler: W,

    hwnd: HWND,
    app: AppContextImpl,
    graphics: Context,

    swapchain: IDXGISwapChain3,
    #[allow(dead_code)]
    target: IDCompositionTarget,
    #[allow(dead_code)]
    visual: IDCompositionVisual,
    swapchain_ready: HANDLE,

    size: WindowSize,

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
    deferred_resize: Option<(WindowSize, Option<f32>)>,
}

impl<W: WindowEventHandler> DxWindowImpl<W> {
    fn new(app: AppContextImpl, user_handler: W, hwnd: HWND) -> Self {
        let (graphics, swapchain, target, visual) = {
            let device = &app.inner.read(); // needs to be here to drop the lock before creating `Self`.
            let graphics = device.dx12.create_context();

            let target = unsafe { device.compositor.CreateTargetForHwnd(hwnd, true) }.unwrap();
            let visual = unsafe { device.compositor.CreateVisual() }.unwrap();
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
                device.dxgi.CreateSwapChainForComposition(
                    &device.dx12.queue.queue,
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

            (graphics, swapchain, target, visual)
        };

        let latency_event = unsafe { swapchain.GetFrameLatencyWaitableObject() };

        let frames_in_flight = [graphics.create_frame(), graphics.create_frame()];
        let draw_list = DrawList::new();

        Self {
            hwnd,
            user_handler,
            app,
            swapchain,
            target,
            visual,
            swapchain_ready: latency_event,
            size: WindowSize::default(),
            graphics,
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

    fn on_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequest => {}
            WindowEvent::Visible(is_visible) => self.is_visible = is_visible,
            WindowEvent::BeginResize => self.is_drag_resizing = true,
            WindowEvent::EndResize => self.is_drag_resizing = false,
            WindowEvent::Resize(new_size) => {
                self.deferred_resize = Some((new_size, None));
            }
        };

        self.user_handler.on_event(event);
    }

    fn on_input(&mut self, input: Input) {
        self.user_handler.on_input(input);
    }

    fn on_os_paint(&mut self) {
        unsafe { WaitForSingleObjectEx(self.swapchain_ready, u32::MAX, true) };

        if let Some((size, flex)) = self.deferred_resize.take() {
            resize_swapchain(&self.swapchain, size.width, size.height, flex, || {
                self.graphics.wait_for_idle()
            });

            tracing::info!("window: resized to {:?}", size);

            self.size = size;
        }

        let mut canvas = {
            let rect = {
                let size = self.size;
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32)
            };

            Canvas::new(&mut self.draw_list, rect)
        };

        let timings = {
            let prev_present_time = {
                let mut stats = DXGI_FRAME_STATISTICS::default();
                unsafe { self.swapchain.GetFrameStatistics(&mut stats) }
                    .ok()
                    .map(|()| Instant::from_ticks(stats.SyncQPCTime as u64))
                    .unwrap_or(Instant::ZERO)
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

        let submit_id = self.graphics.draw(canvas.finish(), frame, image);
        unsafe { self.swapchain.Present(1, 0) }.unwrap();

        self.frame_counter += 1;
        self.prev_submit = Some(submit_id);

        #[cfg(feature = "profile")]
        tracing_tracy::client::frame_mark();
    }

    fn on_vsync(&mut self, _frame_id: FrameId, rate: Option<FramesPerSecond>) {
        self.target_frame_rate = rate;
    }

    fn on_composition_rate(&mut self, _frame_id: FrameId, rate: FramesPerSecond) {
        self.composition_rate = rate;
    }

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
    let width = width as u32;
    let height = height as u32;

    if let Some(flex) = flex {
        let mut desc = Default::default();
        unsafe { swapchain.GetDesc1(&mut desc) }.unwrap();

        if width > desc.Width || height > desc.Height {
            let w = ((width as f32) * flex).min(u16::MAX as f32) as u32;
            let h = ((height as f32) * flex).min(u16::MAX as f32) as u32;

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
