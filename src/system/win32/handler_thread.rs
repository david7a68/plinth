//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

use std::sync::{mpsc::Receiver, Arc};

use parking_lot::RwLock;

use crate::{
    graphics::{Device, GraphicsCommandList, ResizeOp, SubmissionId, Swapchain},
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
        let AppContextImpl { device, sender } = context.clone();

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

        let swapchain = device.create_swapchain(hwnd);
        let command_list = device.create_graphics_command_list();

        let mut state = State {
            device,
            handler,
            shared_state,
            swapchain,
            command_list,
            submission_id: None,
            is_resizing: false,
        };

        // start the command list closed so we don't have to branch on the first
        // iteration of the loop
        state.command_list.finish();

        for event in event_receiver {
            state.on_event(event);
        }

        sender.send(AppMessage::WindowClosed).unwrap();

        // Make sure that the graphics thread has finished with the swapchain
        // before destroying it.
        state.device.wait_for_idle();
    });
}

struct State<W>
where
    W: WindowEventHandler + 'static,
{
    device: Arc<Device>,
    handler: W,
    shared_state: Arc<RwLock<SharedState>>,

    swapchain: Swapchain,
    /// The command list used for drawing operations. Owning it exclusively for
    /// this window might be more memory intensive than a shared command list,
    /// but it's much simpler.
    command_list: GraphicsCommandList,
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
                if self.is_resizing {
                    self.swapchain.resize(
                        &self.device,
                        ResizeOp::Flex {
                            width,
                            height,
                            flex: 2.0,
                        },
                    );
                } else {
                    self.swapchain.resize(&self.device, ResizeOp::Auto);
                }

                let size = Size::new(width as _, height as _);

                self.shared_state.write().size = size;
                self.handler.on_resize(size, Scale::new(scale, scale));
            }
            Event::EndResize => {
                self.is_resizing = false;
                self.swapchain.resize(&self.device, ResizeOp::Auto);

                self.handler.on_end_resize();
            }
            Event::Repaint(timings) => {
                let (image, _) = self.swapchain.get_back_buffer();

                if let Some(submission_id) = self.submission_id {
                    self.device.wait_for_submission(submission_id);
                }

                self.command_list.reset();
                self.command_list.set_render_target(image);
                self.command_list.clear([0.0, 0.0, 0.0, 1.0]);
                self.command_list.finish();

                let submit_id = self.device.submit_graphics_command_list(&self.command_list);
                self.submission_id = Some(submit_id);

                self.swapchain.present(submit_id);

                // TODO: figure out how drawing actually works
                self.handler.on_repaint(timings);
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
