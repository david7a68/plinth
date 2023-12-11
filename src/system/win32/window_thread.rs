//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

use std::{
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
    time::Instant,
};

use parking_lot::RwLock;

use crate::{
    graphics::{Canvas, DrawData, FrameStatistics, Graphics, ResizeOp, SubmissionId, Swapchain},
    math::Scale,
    window::{Window, WindowEventHandler},
};

use super::{
    application::{AppContextImpl, AppMessage},
    window::{Event, SharedState, WindowImpl},
};

/// Spawns a new thread to handle processing of window events.
///
/// Each handler thread can only handle one window and will panic if it receives
/// more than one `Event::Create` message. The lifetime of the spawned thread is
/// tied to the lifetime of the channel receiver and will automatically exit
/// when the channel is closed.
pub(super) fn spawn<W, F>(
    context: AppContextImpl,
    mut constructor: F,
    event_receiver: Receiver<Event>,
) where
    W: WindowEventHandler + 'static,
    F: FnMut(Window) -> W + Send + 'static,
{
    std::thread::spawn(move || {
        let AppContextImpl { graphics, sender } = context.clone();

        let shared_state = Arc::new(RwLock::new(SharedState::default()));

        let (hwnd, handler) = {
            let hwnd = match event_receiver.recv().unwrap() {
                Event::Create(hwnd) => hwnd,
                msg => panic!(
                    "First message must be Event::Create(hwnd). Got {:?} instead.",
                    msg
                ),
            };

            sender.send(AppMessage::WindowCreated).unwrap();
            (
                hwnd,
                constructor(Window::new(WindowImpl {
                    hwnd,
                    context: context.into(),
                    shared_state: shared_state.clone(),
                })),
            )
        };

        State::new(hwnd, handler, shared_state, &graphics).run(&event_receiver);
        sender.send(AppMessage::WindowClosed).unwrap();
    });
}

struct State<'a, W>
where
    W: WindowEventHandler + 'static,
{
    handler: W,
    shared_state: Arc<RwLock<SharedState>>,

    graphics: &'a Arc<Graphics>,
    swapchain: Swapchain,
    draw_data: DrawData,

    target_refresh_rate: f32,

    submission_id: Option<SubmissionId>,
    is_drag_resizing: bool,
}

impl<'a, W> State<'a, W>
where
    W: WindowEventHandler + 'static,
{
    fn new(
        hwnd: windows::Win32::Foundation::HWND,
        handler: W,
        shared_state: Arc<RwLock<SharedState>>,
        graphics: &'a Arc<Graphics>,
    ) -> Self {
        let swapchain = graphics.create_swapchain(hwnd);
        let draw_data = graphics.create_draw_buffer();

        Self {
            handler,
            shared_state,
            graphics,
            swapchain,
            draw_data,
            target_refresh_rate: 0.0,
            submission_id: None,
            is_drag_resizing: false,
        }
    }

    fn run(&mut self, event_receiver: &Receiver<Event>) {
        while self.process_pending::<false>(event_receiver) {
            // 0 means wait for redraw requests
            if self.target_refresh_rate > 0.0 && self.shared_state.read().is_visible {
                self.repaint(); // placeholder, renders every frame
                self.swapchain.wait_for_vsync();
            } else {
                self.process_pending::<true>(event_receiver);
            }
        }
    }

    /// Returns `true` if the window is still open, `false` if it has been
    /// destroyed.
    fn process_pending<const BLOCK: bool>(&mut self, event_receiver: &Receiver<Event>) -> bool {
        if BLOCK {
            // only fails if the channel is closed
            let Ok(event) = event_receiver.recv() else {
                return false;
            };

            // This is so that we don't try to render after returning, even
            // though the channel is still open.
            if !self.on_event(event) {
                return false;
            }
        }

        loop {
            match event_receiver.try_recv() {
                Ok(event) => {
                    if !self.on_event(event) {
                        break false;
                    }
                }
                Err(e) => match e {
                    TryRecvError::Empty => break true,
                    TryRecvError::Disconnected => break false,
                },
            }
        }
    }

    fn on_event(&mut self, event: Event) -> bool {
        // default return true, explicitly return false if we want to exit
        match event {
            Event::Create(_) => {
                panic!("Window already created");
            }
            Event::CloseRequest => {
                self.handler.on_close_request();
            }
            Event::Destroy => {
                self.handler.on_destroy();
                return false;
            }
            Event::Visible(is_visible) => {
                self.handler.on_visible(is_visible);
            }
            Event::BeginResize => {
                self.is_drag_resizing = true;
                self.handler.on_begin_resize();
            }
            Event::Resize {
                width,
                height,
                scale,
            } => {
                let op = if self.is_drag_resizing {
                    ResizeOp::Flex {
                        width,
                        height,
                        flex: 2.0,
                    }
                } else {
                    ResizeOp::Auto
                };

                self.graphics.resize_swapchain(&mut self.swapchain, op);

                let size = (width, height).into();

                {
                    let mut state = self.shared_state.write();
                    state.size = size;
                    state.is_visible = width > 0 && height > 0;
                }

                self.handler.on_resize(size, Scale::new(scale, scale));
            }
            Event::EndResize => {
                self.is_drag_resizing = false;
                self.graphics
                    .resize_swapchain(&mut self.swapchain, ResizeOp::Auto);

                self.handler.on_end_resize();
            }
            Event::SetAnimationFrequency(freq) => {
                self.target_refresh_rate = freq;
            }
            Event::Repaint => {
                // if we're already animating, don't repaint now because we'll
                // be doing it once event processing is complete.
                if self.target_refresh_rate == 0.0 {
                    self.repaint();
                }
            }
            Event::PointerMove(location) => {
                let location = location.into();
                let delta = {
                    let mut shared_state = self.shared_state.write();
                    shared_state.pointer_location = Some(location);

                    if let Some(last_cursor_pos) = shared_state.pointer_location {
                        location - last_cursor_pos
                    } else {
                        (0.0, 0.0).into()
                    }
                };

                self.handler.on_pointer_move(location, delta.into());
            }
            Event::PointerLeave => {
                self.handler.on_pointer_leave();
            }
            Event::MouseButton(button, state, location) => {
                self.handler.on_mouse_button(button, state, location.into());
            }
            Event::Scroll(axis, delta) => {
                self.handler.on_scroll(axis, delta);
            }
        }

        true
    }

    fn repaint(&mut self) {
        let (image, _) = self.swapchain.get_back_buffer();

        if let Some(submission_id) = self.submission_id {
            self.graphics.wait_for_submission(submission_id);
        }

        self.draw_data.reset();
        let rect = self.shared_state.read().size.into();
        let mut canvas = Canvas::new(&mut self.draw_data, rect, image);

        let dummy_stats = FrameStatistics {
            prev_max_frame_budget: Default::default(),
            prev_adj_frame_budget: Default::default(),
            prev_cpu_render_time: Default::default(),
            prev_gpu_render_time: Default::default(),
            prev_all_render_time: Default::default(),
            prev_present_time: Instant::now(),
            next_estimated_present: Instant::now(),
        };

        self.handler.on_repaint(&mut canvas, &dummy_stats);

        // copy geometry from the geometry buffer to a temp buffer and
        // close the command list
        self.draw_data.finish();

        let submit_id = self.graphics.draw(&self.draw_data);

        self.submission_id = Some(submit_id);
        self.swapchain.present(submit_id);
    }
}

impl<W> Drop for State<'_, W>
where
    W: WindowEventHandler + 'static,
{
    fn drop(&mut self) {
        self.graphics.wait_for_idle();
    }
}
