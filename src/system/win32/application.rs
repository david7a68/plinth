use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use crate::{
    application::GraphicsConfig,
    graphics::{self},
    window::{WindowEventHandler, WindowSpec},
};

use super::window::spawn_window;

#[derive(Debug)]
pub(super) enum AppMessage {
    WindowCreated,
    WindowClosed,
}

pub struct Application {
    device: Arc<graphics::Device>,
    sender: Sender<AppMessage>,
    receiver: Receiver<AppMessage>,
}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        // TODO: this bound is nonsense. actually figure out what it should be.
        let (sender, receiver) = std::sync::mpsc::channel();

        let device = Arc::new(graphics::Device::new(graphics));

        Self {
            device,
            sender,
            receiver,
        }
    }

    pub fn context(&self) -> AppContext {
        AppContext::new(self.device.clone(), self.sender.clone())
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        spawn_window(self.context(), spec, constructor);
    }

    pub fn run(&mut self) {
        let mut num_windows = 0;

        while let Ok(msg) = self.receiver.recv() {
            println!("Received message: {:?}", msg);
            match msg {
                AppMessage::WindowCreated => num_windows += 1,
                AppMessage::WindowClosed => num_windows -= 1,
            }

            // This is redundant so long as only windows hold AppContexts.
            if num_windows == 0 {
                break;
            }
        }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        // wait for graphics thread to exit
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub(crate) device: Arc<graphics::Device>,
    pub(super) sender: Sender<AppMessage>,
}

impl AppContext {
    fn new(device: Arc<graphics::Device>, sender: Sender<AppMessage>) -> Self {
        Self { sender, device }
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        spawn_window(self.clone(), spec, constructor);
    }
}
