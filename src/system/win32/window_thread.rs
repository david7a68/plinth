//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

use std::sync::{mpsc::Receiver, Arc};

use parking_lot::RwLock;

use crate::{
    graphics::{Canvas, DrawData, Graphics, ResizeOp, SubmissionId, Swapchain},
    math::{Scale, Size},
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

        let swapchain = graphics.create_swapchain(hwnd);
        let draw_data = graphics.create_draw_buffer();

        let mut state = State {
            handler,
            shared_state,
            graphics,
            swapchain,
            draw_data,
            submission_id: None,
            is_resizing: false,
        };

        for event in event_receiver {
            state.on_event(event);
        }

        sender.send(AppMessage::WindowClosed).unwrap();

        // Make sure that the graphics thread has finished with the swapchain
        // before destroying it.
        state.graphics.wait_for_idle();
    });
}

struct State<W>
where
    W: WindowEventHandler + 'static,
{
    handler: W,
    shared_state: Arc<RwLock<SharedState>>,

    graphics: Arc<Graphics>,
    swapchain: Swapchain,
    draw_data: DrawData,

    submission_id: Option<SubmissionId>,
    is_resizing: bool,
}

impl<W> State<W>
where
    W: WindowEventHandler + 'static,
{
    fn on_event(&mut self, event: Event) {
        match event {
            Event::Create(_) => {
                panic!("Window already created");
            }
            Event::CloseRequest => {
                self.handler.on_close_request();
            }
            Event::Destroy => {
                self.handler.on_destroy();
            }
            Event::Visible(is_visible) => {
                self.handler.on_visible(is_visible);
            }
            Event::BeginResize => {
                self.is_resizing = true;
                self.handler.on_begin_resize();
            }
            Event::Resize {
                width,
                height,
                scale,
            } => {
                let op = if self.is_resizing {
                    ResizeOp::Flex {
                        width,
                        height,
                        flex: 2.0,
                    }
                } else {
                    ResizeOp::Auto
                };

                self.graphics.resize_swapchain(&mut self.swapchain, op);

                let size = Size::new(width as _, height as _);

                self.shared_state.write().size = size;
                self.handler.on_resize(size, Scale::new(scale, scale));
            }
            Event::EndResize => {
                self.is_resizing = false;
                self.graphics
                    .resize_swapchain(&mut self.swapchain, ResizeOp::Auto);

                self.handler.on_end_resize();
            }
            Event::Repaint(timings) => {
                let (image, _) = self.swapchain.get_back_buffer();

                if let Some(submission_id) = self.submission_id {
                    self.graphics.wait_for_submission(submission_id);
                }

                self.draw_data.reset();
                let rect = self.shared_state.read().size.into();
                let mut canvas = Canvas::new(&mut self.draw_data, rect, image);

                self.handler.on_repaint(&mut canvas, timings);

                // copy geometry from the geometry buffer to a temp buffer and
                // close the command list
                self.draw_data.finish();

                let submit_id = self.graphics.draw(&self.draw_data);

                self.submission_id = Some(submit_id);
                self.swapchain.present(submit_id);
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
            Event::Scroll(axis, delta) => self.handler.on_scroll(axis, delta),
        }
    }
}
