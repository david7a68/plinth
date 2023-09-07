use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use crate::{
    graphics::{Device, GraphicsConfig, ResizeOp, Swapchain},
    shell::WindowHandle,
    vsync::VSyncSource,
};

#[derive(Clone, Copy)]
pub struct WindowId(u64);

pub enum Message {
    NewWindow(WindowHandle, Sender<Response>),
    DestroyWindow(WindowId),
    ResizeWindow(WindowId, ResizeOp, Sender<Response>),
    EnableVSync(WindowId),
    DisableVSync(WindowId),
    ForceDraw(WindowId, Sender<Response>),
    VSync,
    Exit,
}

pub enum Response {
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

    fn draw(&self, device: &mut Device) {
        let (image, _) = self.swapchain.get_back_buffer();
        let canvas = device.create_canvas(image);
        device.draw_canvas(canvas);
        self.swapchain.present();
    }
}

pub struct RenderThread {
    joiner: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl RenderThread {
    pub fn spawn(config: GraphicsConfig) -> (Self, RenderThreadProxy) {
        let (sender, receiver) = channel();
        let vsync_sender = sender.clone();

        let thread = std::thread::spawn(move || {
            let mut thread = Thread {
                vsync: VSyncSource::new(vsync_sender),
                device: Device::new(&config),
                windows: Vec::new(),
                free_windows: Vec::new(),
                num_vsync: 0,
            };

            thread.vsync.stop();

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
                        Message::ForceDraw(id, reply) => thread.force_draw(id, reply),
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

                    for output in &thread.windows {
                        if let Some(output) = output {
                            if output.is_vsync_enabled {
                                output.draw(&mut thread.device);
                            }
                        }
                    }
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

struct Thread {
    vsync: VSyncSource,
    device: Device,

    windows: Vec<Option<Output>>,
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
            self.windows[id as usize] = Some(output);
            WindowId(id as u64)
        } else {
            let id = self.windows.len();
            self.windows.push(Some(output));
            WindowId(id as u64)
        };

        reply.send(Response::NewWindow(id)).unwrap();
    }

    #[tracing::instrument(skip_all)]
    fn destroy_window(&mut self, id: WindowId) {
        if let Some(output) = self.windows[id.0 as usize].take() {
            if output.is_vsync_enabled {
                self.num_vsync -= 1;
            }

            self.free_windows.push(id.0 as u32);
        }
    }

    #[tracing::instrument(skip_all)]
    fn resize_window(&mut self, id: WindowId, op: ResizeOp, reply: Sender<Response>) {
        self.windows[id.0 as usize].as_mut().unwrap().resize(op);
        reply.send(Response::ResizeWindow).unwrap();
    }

    #[tracing::instrument(skip_all)]
    fn enable_vsync(&mut self, id: WindowId) {
        self.windows[id.0 as usize]
            .as_mut()
            .unwrap()
            .is_vsync_enabled = true;

        self.num_vsync += 1;

        if self.num_vsync == 1 {
            self.vsync.start();
        }
    }

    #[tracing::instrument(skip_all)]
    fn disable_vsync(&mut self, id: WindowId) {
        self.windows[id.0 as usize]
            .as_mut()
            .unwrap()
            .is_vsync_enabled = false;

        self.num_vsync = self.num_vsync.saturating_sub(1);

        if self.num_vsync == 0 {
            self.vsync.stop();
        }
    }

    #[tracing::instrument(skip_all)]
    fn force_draw(&mut self, id: WindowId, reply: Sender<Response>) {
        let output = self.windows[id.0 as usize].as_mut().unwrap();
        output.draw(&mut self.device);
        reply.send(Response::Repaint).unwrap();
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
            .send(Message::ForceDraw(id, self.response_render.clone()))
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
