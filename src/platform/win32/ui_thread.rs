// todo: account for CPU time when scheduling the next repaint

use std::sync::{mpsc::Receiver, Arc};

use parking_lot::RwLock;
use windows::Win32::Foundation::HWND;

use crate::{
    application::AppContext,
    frame::{FramesPerSecond, RedrawRequest},
    graphics::{Canvas, FrameInfo},
    math::Rect,
    platform::{
        dx12::{Context, Frame},
        gfx::{Context as _, Device as _, DrawList, SubmitId},
        WindowImpl,
    },
    time::Instant,
    window::WindowEvent,
    Input, Window, WindowEventHandler, WindowSize,
};

use super::{
    swapchain::Swapchain,
    vsync::VsyncRequest,
    window::{Control, WindowState},
    AppContextImpl,
};

#[derive(Debug, PartialEq, Eq)]
enum Tick {
    Continue,
    Quit,
}

#[derive(Debug)]
pub enum UiEvent {
    New(HWND),
    Quit,
    Input(Input),
    Window(WindowEvent),
    ControlEvent(Control),
}

pub fn spawn_ui_thread<W, F>(
    context: AppContextImpl,
    constructor: F,
    ui_receiver: Receiver<UiEvent>,
) where
    W: WindowEventHandler,
    F: FnMut(Window) -> W + Send + 'static,
{
    std::thread::spawn(move || run_ui_loop(context, constructor, ui_receiver));
}

pub fn run_ui_loop<W, F>(
    context: AppContextImpl,
    mut constructor: F,
    ui_receiver: Receiver<UiEvent>,
) where
    W: WindowEventHandler,
    F: FnMut(Window) -> W + Send + 'static,
{
    let hwnd = match ui_receiver.recv() {
        Ok(UiEvent::New(hwnd)) => hwnd,
        Ok(e) => {
            panic!("First message to UI loop must be UiEvent::New(HWND)! Got {e:?} instead.");
        }
        Err(_) => {
            tracing::warn!("Started a UI loop with a closed event channel.");
            return;
        }
    };

    let shared_state = Arc::new(RwLock::new(WindowState::default()));
    let handler = constructor(Window::new(WindowImpl::new(
        hwnd,
        shared_state.clone(),
        AppContext {
            inner: context.clone(),
        },
    )));

    let graphics = context.inner.read().dx12.create_context();
    let swapchain = context.create_swapchain(hwnd);
    let frames_in_flight = [graphics.create_frame(), graphics.create_frame()];
    let draw_list = DrawList::new();

    let mut render_state = RenderState {
        hwnd,
        handler,
        graphics,
        shared_state,
        size: WindowSize::default(),
        swapchain,
        draw_list,
        frames_in_flight,
        prev_submit: None,
        frame_counter: 0,
        target_frame_rate: None,
        is_visible: false,
        composition_rate: FramesPerSecond::ZERO,
        is_drag_resizing: false,
        need_repaint: false,
        deferred_resize: None,
    };

    let request_vsync = |request| context.vsync_sender.send(request).unwrap();

    // blocking wait for events
    while let Ok(event) = ui_receiver.recv() {
        let mut tick_result = render_state.tick(event, &request_vsync);

        // processed any queued events
        while let Ok(event) = ui_receiver.try_recv() {
            tick_result = render_state.tick(event, &request_vsync);
        }

        if tick_result == Tick::Quit {
            tracing::info!("window: quit");
            break;
        }

        render_state.repaint_if_needed();
    }
}

struct RenderState<W: WindowEventHandler> {
    hwnd: HWND,
    handler: W,
    graphics: Context,
    shared_state: Arc<RwLock<WindowState>>,

    size: WindowSize,
    swapchain: Swapchain,
    draw_list: DrawList,
    frames_in_flight: [Frame; 2],
    prev_submit: Option<SubmitId>,

    composition_rate: FramesPerSecond,
    target_frame_rate: Option<FramesPerSecond>,

    frame_counter: u64,
    is_visible: bool,
    is_drag_resizing: bool,

    /// Flag that the window needs to be repainted this frame. Repaint
    /// notifications (vsync or from the OS) get marshalled together so that we
    /// don't repaint several times in a single vblank.
    need_repaint: bool,

    /// A resize event. Deferred until repaint to consolidate graphics work and
    /// in case multiple resize events are received in a single frame.
    deferred_resize: Option<(WindowSize, Option<f32>)>,
}

impl<W: WindowEventHandler> RenderState<W> {
    fn tick<F: Fn(VsyncRequest)>(&mut self, event: UiEvent, request_vsync: &F) -> Tick {
        let mut req_vsync = |request: RedrawRequest| match request {
            RedrawRequest::Idle => request_vsync(VsyncRequest::Idle(self.hwnd)),
            RedrawRequest::Once => self.need_repaint = true,
            RedrawRequest::AtFrame(frame) => request_vsync(VsyncRequest::AtFrame(self.hwnd, frame)),
            RedrawRequest::AtFrameRate(rate) => {
                request_vsync(VsyncRequest::AtFrameRate(self.hwnd, rate))
            }
        };

        match event {
            UiEvent::New{..} => unreachable!("Received an UiEvent::New after the UI thread has been initialized! This should only be send once."),
            UiEvent::Quit => return Tick::Quit,
            UiEvent::Input(input) => self.handler.on_input(input),
            UiEvent::Window(event) => {
                match event {
                    WindowEvent::CloseRequest => {},
                    WindowEvent::Visible(is_visible) => self.is_visible = is_visible,
                    WindowEvent::BeginResize => self.is_drag_resizing = true,
                    WindowEvent::EndResize => self.is_drag_resizing = false,
                    WindowEvent::Resize(new_size) => {
                        self.deferred_resize = Some((new_size, None));
                    },
                }

                self.handler.on_event(event);
            },
            UiEvent::ControlEvent(event) => match event {
                Control::Redraw(req) => {
                        req_vsync(req);
                },
                Control::OsRepaint => {
                    self.need_repaint = true;
                },
                Control::VSync(_frame_id, rate) => {
                    // todo: what to do about frame_id? -dz
                    self.need_repaint = true;
                    self.target_frame_rate = rate;
                },
                Control::VSyncRateChanged(_frame_id, rate) => {
                    // todo: what to do about frame_id? -dz
                    self.composition_rate = rate;
                }
            },
        }

        Tick::Continue
    }

    //// Repaint if the `needs_repaint` flag has been set.
    fn repaint_if_needed(&mut self) {
        if !self.need_repaint {
            return;
        }

        self.need_repaint = false;
        self.swapchain.wait_for_present();

        if let Some((size, flex)) = self.deferred_resize.take() {
            self.swapchain.resize(size.width, size.height, flex, || {
                self.graphics.wait_for_idle()
            });

            tracing::info!("window: resized to {:?}", size);

            self.size = size;
            self.shared_state.write().size = size;
        }

        let mut canvas = {
            let rect = {
                let size = self.size;
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32)
            };

            Canvas::new(&mut self.draw_list, rect)
        };

        let timings = {
            let prev_present_time = self.swapchain.prev_present_time().unwrap_or(Instant::ZERO);

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

        self.handler.on_repaint(&mut canvas, &timings);

        let (image, _) = self.swapchain.get_back_buffer();
        let frame = &mut self.frames_in_flight[(self.frame_counter % 2) as usize];

        let submit_id = self.graphics.draw(canvas.finish(), frame, image);
        self.swapchain.present();

        self.frame_counter += 1;
        self.prev_submit = Some(submit_id);

        #[cfg(feature = "profile")]
        tracing_tracy::client::frame_mark();
    }
}

impl<W: WindowEventHandler> Drop for RenderState<W> {
    fn drop(&mut self) {
        tracing::info!("window: dropping");

        self.graphics.wait_for_idle();
    }
}
