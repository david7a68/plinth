mod vsync;
mod worker;

use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use crate::shell::WindowHandle;

use self::{vsync::VSyncSource, worker::Worker};

use super::{Device, GraphicsConfig, ResizeOp, Swapchain};

#[derive(Clone, Copy)]
pub struct WindowId(u64);

enum Message {
    // Window messages
    NewWindow(WindowHandle, Sender<Response>),
    DestroyWindow(WindowId),
    ResizeWindow(WindowId, ResizeOp, Sender<Response>),
    EnableVSync(WindowId),
    DisableVSync(WindowId),
    RepaintNow(WindowId, Sender<Response>),

    // VSync messages
    VSync,

    // Worker thread messages
    WorkerDone(WindowId),

    // Exit message
    Exit,
}

enum Response {
    NewWindow(WindowId),
    ResizeWindow,
    Repaint,
}

struct Output {
    swapchain: Swapchain,

    is_vsync_enabled: bool,
}

impl Output {
    fn resize(&mut self, op: ResizeOp) {
        self.swapchain.resize(op);
    }

    #[tracing::instrument(skip_all)]
    fn repaint(&mut self, device: &mut Device) {
        let (image, _) = self.swapchain.get_back_buffer();
        let canvas = device.create_canvas(image);
        let submit_id = device.draw_canvas(canvas);
        self.swapchain.present(submit_id);
    }
}

/// Lifetime control object for the render thread. Drop this to stop the
/// renderer.
pub struct RenderThread {
    joiner: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl RenderThread {
    pub fn spawn(config: GraphicsConfig) -> (Self, RenderThreadProxy) {
        let (sender, receiver) = channel();
        let worker_sender = sender.clone();

        let thread = std::thread::spawn(move || {
            // todo: tracy name thread

            let mut thread = Thread {
                vsync: VSyncSource::new(worker_sender.clone()),
                worker: Worker::new(worker_sender),
                device: Device::new(&config),
                windows: Vec::new(),
                free_windows: Vec::new(),
                num_vsync: 0,
            };

            loop {
                // Wait for a message.
                let Ok(message) = receiver.recv() else {
                    break;
                };

                // Accumulate vsync in case rendering is behind and there are
                // multiple vsync messages.
                let mut is_vsync = false;

                let mut message = Some(message);
                while let Some(msg) = message {
                    match msg {
                        Message::NewWindow(handle, reply) => thread.new_window(handle, reply),
                        Message::DestroyWindow(id) => thread.destroy_window(id),
                        Message::ResizeWindow(id, op, reply) => thread.resize_window(id, op, reply),
                        Message::EnableVSync(id) => thread.enable_vsync(id),
                        Message::DisableVSync(id) => thread.disable_vsync(id),
                        Message::RepaintNow(id, reply) => thread.force_draw(id, reply),
                        Message::WorkerDone(id) => thread.worker_done(id),
                        Message::VSync => {
                            is_vsync = true;
                        }
                        Message::Exit => return,
                    }
                    message = receiver.try_recv().ok();
                }

                if is_vsync {
                    #[cfg(feature = "profile")]
                    tracing_tracy::client::frame_mark();

                    thread.draw_vsync();
                }
            }
        });

        (
            Self {
                joiner: Some(thread),
                sender: sender.clone(),
            },
            RenderThreadProxy::new(sender),
        )
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.sender.send(Message::Exit).unwrap();
        self.joiner.take().unwrap().join().unwrap();
    }
}

enum OutputLocation {
    /// The output slot is empty.
    ///
    /// This may be set even though the output is being destroyed on a worker
    /// thread.
    None,

    /// The output is on the render thread.
    Local(Output),

    /// Thes output is on a worker thread. Wait on the receiver to retrieve the
    /// output.
    Worker(Receiver<Output>, Option<(Sender<Response>, Response)>),
}

struct Thread {
    vsync: VSyncSource,
    worker: Worker,
    device: Device,

    windows: Vec<OutputLocation>,
    free_windows: Vec<u32>,

    num_vsync: usize,
}

impl Thread {
    #[tracing::instrument(skip_all)]
    fn new_window(&mut self, handle: WindowHandle, reply: Sender<Response>) {
        let output = Output {
            swapchain: self.device.create_swapchain(handle.hwnd().unwrap()),
            is_vsync_enabled: false,
        };

        let id = if let Some(id) = self.free_windows.pop() {
            self.windows[id as usize] = OutputLocation::Local(output);
            WindowId(id as u64)
        } else {
            let id = self.windows.len();
            self.windows.push(OutputLocation::Local(output));
            WindowId(id as u64)
        };

        reply.send(Response::NewWindow(id)).unwrap();
    }

    #[tracing::instrument(skip_all)]
    fn destroy_window(&mut self, id: WindowId) {
        let output = self.output_to_worker(id, OutputLocation::None);
        if output.is_vsync_enabled {
            self.num_vsync -= 1;
        }

        self.worker.destroy_output(output);

        self.free_windows.push(id.0 as u32);
    }

    #[tracing::instrument(skip_all)]
    fn resize_window(&mut self, id: WindowId, op: ResizeOp, reply: Sender<Response>) {
        let (sender, receiver) = channel();

        let new_location = OutputLocation::Worker(receiver, Some((reply, Response::ResizeWindow)));
        let output = self.output_to_worker(id, new_location);
        self.worker.resize_output(id, output, op, sender);
    }

    #[tracing::instrument(skip_all)]
    fn enable_vsync(&mut self, id: WindowId) {
        let output = Self::borrow_output(&mut self.windows, id);
        output.is_vsync_enabled = true;

        self.num_vsync += 1;
        if self.num_vsync == 1 {
            self.vsync.next();
        }
    }

    #[tracing::instrument(skip_all)]
    fn disable_vsync(&mut self, id: WindowId) {
        let output = Self::borrow_output(&mut self.windows, id);
        output.is_vsync_enabled = false;

        self.num_vsync = self.num_vsync.saturating_sub(1);
    }

    #[tracing::instrument(skip_all)]
    fn draw_vsync(&mut self) {
        for i in 0..self.windows.len() {
            if let Some(output) = Self::try_borrow_output(&mut self.windows, WindowId(i as u64)) {
                if output.is_vsync_enabled {
                    output.repaint(&mut self.device);
                }
            }
        }

        if self.num_vsync > 0 {
            self.vsync.next();
        }
    }

    #[tracing::instrument(skip_all)]
    fn force_draw(&mut self, id: WindowId, reply: Sender<Response>) {
        let output = Self::borrow_output(&mut self.windows, id);
        output.repaint(&mut self.device);
        reply.send(Response::Repaint).unwrap();
    }

    #[tracing::instrument(skip_all)]
    fn worker_done(&mut self, id: WindowId) {
        let result = std::mem::replace(&mut self.windows[id.0 as usize], OutputLocation::None);

        match result {
            OutputLocation::None => panic!("Worker thread finished on destroyed output"),
            OutputLocation::Local(_) => panic!("Worker thread finished on local output"),
            OutputLocation::Worker(receiver, response) => {
                let output = receiver
                    .try_recv()
                    .expect("Worker thread sent WorkDone message but no output available");

                debug_assert!(
                    receiver.try_recv().is_err(),
                    "Worker thread sent multiple outputs when only one was expected"
                );

                self.windows[id.0 as usize] = OutputLocation::Local(output);

                if let Some((reply, message)) = response {
                    reply.send(message).unwrap();
                }
            }
        }
    }

    fn output_to_worker(&mut self, id: WindowId, value: OutputLocation) -> Output {
        match std::mem::replace(&mut self.windows[id.0 as usize], value) {
            OutputLocation::None => panic!("Output has been destroyed"),
            OutputLocation::Local(output) => output,
            OutputLocation::Worker(_, _) => panic!("Output is already being used by worker thread"),
        }
    }

    fn borrow_output(outputs: &mut [OutputLocation], id: WindowId) -> &mut Output {
        let slot = &mut outputs[id.0 as usize];

        match slot {
            OutputLocation::None => panic!("Borrowing destroyed output"),
            OutputLocation::Local(output) => output,
            OutputLocation::Worker(_, _) => {
                panic!("Cannot borrow output that is being used by worker thread")
            }
        }
    }

    fn try_borrow_output(outputs: &mut [OutputLocation], id: WindowId) -> Option<&mut Output> {
        if let OutputLocation::Local(output) = &mut outputs[id.0 as usize] {
            Some(output)
        } else {
            None
        }
    }
}

pub struct RenderThreadProxy {
    message_sender: Sender<Message>,
    response_render: Sender<Response>,
    response_receiver: Receiver<Response>,
}

impl RenderThreadProxy {
    fn new(sender: Sender<Message>) -> Self {
        let (response_render, response_receiver) = channel();

        Self {
            message_sender: sender,
            response_render,
            response_receiver,
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn new_window(&mut self, handle: WindowHandle) -> WindowId {
        self.message_sender
            .send(Message::NewWindow(handle, self.response_render.clone()))
            .unwrap();

        match self.response_receiver.recv().unwrap() {
            Response::NewWindow(id) => id,
            _ => panic!("Unexpected response from render thread"),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn destroy_window(&mut self, id: WindowId) {
        self.message_sender
            .send(Message::DestroyWindow(id))
            .unwrap();
    }

    #[tracing::instrument(skip_all)]
    pub fn resize_window(&mut self, id: WindowId, op: ResizeOp) {
        self.message_sender
            .send(Message::ResizeWindow(id, op, self.response_render.clone()))
            .unwrap();

        match self.response_receiver.recv().unwrap() {
            Response::ResizeWindow => (),
            _ => panic!("Unexpected response from render thread"),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn enable_vsync(&mut self, id: WindowId) {
        self.message_sender.send(Message::EnableVSync(id)).unwrap();
    }

    #[tracing::instrument(skip_all)]
    pub fn disable_vsync(&mut self, id: WindowId) {
        self.message_sender.send(Message::DisableVSync(id)).unwrap();
    }

    #[tracing::instrument(skip_all)]
    pub fn force_draw(&mut self, id: WindowId) {
        self.message_sender
            .send(Message::RepaintNow(id, self.response_render.clone()))
            .unwrap();

        match self.response_receiver.recv().unwrap() {
            Response::Repaint => (),
            _ => panic!("Unexpected response from render thread"),
        }
    }
}

impl Clone for RenderThreadProxy {
    fn clone(&self) -> Self {
        let (response_render, response_receiver) = channel();

        Self {
            message_sender: self.message_sender.clone(),
            response_render,
            response_receiver,
        }
    }
}
