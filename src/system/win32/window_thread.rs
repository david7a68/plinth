//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

// reimplement cpu frame timing to account for long draw times
//     use running average of N frames
// allocate 2 frames in flight, split draw data into DrawList (cpu) and DrawBuffer (gpu)

use std::sync::{
    mpsc::{Receiver, TryRecvError},
    Arc,
};

use parking_lot::RwLock;

use crate::{
    graphics::{
        backend::{ResizeOp, SubmissionId, Swapchain},
        Canvas, DrawData, FrameInfo, FramesPerSecond, Graphics, RefreshRate,
    },
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
    mode: Mode,
    handler: W,
    shared_state: Arc<RwLock<SharedState>>,

    graphics: &'a Arc<Graphics>,
    swapchain: Swapchain,
    draw_data: DrawData,

    submission_id: Option<SubmissionId>,

    requested_refresh_rate: FramesPerSecond,
    actual_refresh_rate: FramesPerSecond,
    vblanks_per_frame: u32,

    is_drag_resizing: bool,
    need_repaint: bool,
    repaint_now: bool,
}

enum Mode {
    Idle,
    Animating { vblanks_since_last_present: u32 },
    Closed,
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

        shared_state.write().refresh_rate = {
            let rate = swapchain.output().refresh_rate();

            RefreshRate {
                min: rate.min,
                max: rate.max,
                now: FramesPerSecond(0.0),
            }
        };

        Self {
            mode: Mode::Idle,
            handler,
            shared_state,
            graphics,
            swapchain,
            draw_data,
            submission_id: None,
            requested_refresh_rate: FramesPerSecond(0.0),
            actual_refresh_rate: FramesPerSecond(0.0),
            vblanks_per_frame: 0,
            is_drag_resizing: false,
            need_repaint: false,
            repaint_now: false,
        }
    }

    fn run(&mut self, event_receiver: &Receiver<Event>) {
        loop {
            match self.mode {
                Mode::Idle => self.run_idle(event_receiver),
                Mode::Animating { .. } => self.run_animating(event_receiver),
                Mode::Closed => break,
            }
        }
    }

    fn run_idle(&mut self, event_receiver: &Receiver<Event>) {
        while self.process_pending::<true>(event_receiver) {
            match &self.mode {
                Mode::Idle => {
                    if self.need_repaint {
                        self.repaint();
                    }
                }
                _ => {
                    break;
                }
            }
        }
    }

    fn run_animating(&mut self, event_receiver: &Receiver<Event>) {
        while self.process_pending::<false>(event_receiver) {
            let Mode::Animating {
                vblanks_since_last_present,
            } = &mut self.mode
            else {
                break;
            };

            *vblanks_since_last_present += 1;

            if *vblanks_since_last_present >= self.vblanks_per_frame {
                *vblanks_since_last_present = 0;
                self.need_repaint = true;
            }

            if self.need_repaint {
                self.repaint();
            }

            self.swapchain.output().wait_for_vsync();
        }
    }

    /// Returns `true` if the window is still open, `false` if it has been
    /// destroyed.
    #[tracing::instrument(skip(self, event_receiver))]
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

    #[tracing::instrument(skip(self))]
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
                self.mode = Mode::Closed;
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
                    ResizeOp::Fixed { width, height }
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

                let size = self.shared_state.read().size;
                self.graphics.resize_swapchain(
                    &mut self.swapchain,
                    ResizeOp::Fixed {
                        width: size.width as u32,
                        height: size.height as u32,
                    },
                );

                self.handler.on_end_resize();
            }
            Event::SetAnimationFrequency(freq) => {
                if freq > 0.0 {
                    let vblanks_per_frame = (self.swapchain.output().refresh_rate().now / freq)
                        .floor()
                        .max(1.0);

                    let refresh_rate =
                        self.swapchain.output().refresh_rate().now / vblanks_per_frame;

                    self.vblanks_per_frame = vblanks_per_frame as u32;
                    self.actual_refresh_rate = refresh_rate;
                    self.requested_refresh_rate = freq;

                    self.mode = Mode::Animating {
                        // force a repaint on the first frame
                        vblanks_since_last_present: self.vblanks_per_frame,
                    };
                } else {
                    self.mode = Mode::Idle;
                }
            }
            Event::Repaint => {
                self.need_repaint = true;
                self.repaint_now = true;
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

    #[tracing::instrument(skip(self))]
    fn repaint(&mut self) {
        let stats = {
            let prev_present_time = self.swapchain.present_statistics().prev_present_time;
            let next_present_time = self.swapchain.output().refresh_rate().now.frame_time()
                * self.vblanks_per_frame as f64;

            FrameInfo {
                frame_rate: self.actual_refresh_rate,
                prev_present_time: prev_present_time,
                next_present_time: prev_present_time + next_present_time.0,
            }
        };

        if let Some(submission_id) = self.submission_id {
            self.graphics.wait_for_submission(submission_id);
        }

        let draw_data = {
            self.draw_data.reset();
            let rect = self.shared_state.read().size.into();

            let (image, _) = self.swapchain.get_back_buffer();
            let mut canvas = Canvas::new(&mut self.draw_data, rect, image);
            self.handler.on_repaint(&mut canvas, &stats);
            canvas.finish()
        };

        draw_data.finish();

        // copy geometry from the geometry buffer to a temp buffer and
        let submit_id = self.graphics.draw(draw_data);

        self.submission_id = Some(submit_id);

        self.swapchain.present(submit_id, 1);

        self.need_repaint = false;
        self.repaint_now = false;

        #[cfg(feature = "profile")]
        {
            tracing_tracy::client::frame_mark();
        }
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
