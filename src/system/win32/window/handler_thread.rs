use std::sync::{mpsc::Receiver, Arc};

use crate::{
    graphics::{Device, GraphicsCommandList, ResizeOp, SubmissionId, Swapchain},
    math::{Scale, Size},
    system::{win32::application::AppMessage, AppContext},
    window::WindowEventHandler,
};

use super::{Event, Window};

/// Spawns a new thread to handle processing of window events. One handler
/// thread is created for each window.
pub fn spawn<W, F>(context: AppContext, mut constructor: F, event_receiver: Receiver<Event>)
where
    W: WindowEventHandler + 'static,
    F: FnMut(crate::window::Window) -> W + Send + 'static,
{
    std::thread::spawn(move || {
        let AppContext { device, sender } = context.clone();

        let (hwnd, handler) = {
            let Event::Create(hwnd) = event_receiver.recv().unwrap() else {
                panic!("First message must be Event::Create(hwnd)");
            };
            sender.send(AppMessage::WindowCreated).unwrap();
            (
                hwnd,
                constructor(crate::window::Window::new(Window {
                    hwnd,
                    context: context.into(),
                })),
            )
        };

        let swapchain = device.create_swapchain(hwnd);
        let command_list = device.create_graphics_command_list();

        let mut state = State {
            device,
            handler,
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
    swapchain: Swapchain,
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
                            flex: 1.5,
                        },
                    );
                } else {
                    self.swapchain.resize(&self.device, ResizeOp::Auto);
                }

                self.handler
                    .on_resize(Size::new(width as _, height as _), Scale::new(scale, scale));
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
            Event::PointerMove(location, delta) => {
                self.handler
                    .on_pointer_move(location.retype(), delta.retype());
            }
            Event::Scroll(_, _) => {
                self.handler.on_close_request();
            }
        }
    }
}
