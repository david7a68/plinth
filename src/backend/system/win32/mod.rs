mod window;
pub use window::Window;

use crossbeam::channel::{Receiver, Sender};

use crate::{
    application::GraphicsConfig,
    window::{WindowEventHandler, WindowSpec},
};

use self::window::spawn_window_thread;

#[derive(Debug)]
pub(self) enum AppMessage {
    WindowCreated,
    WindowClosed,
}

#[derive(Clone)]
pub struct Application {
    context: AppContext,
    receiver: Receiver<AppMessage>,
}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        // TODO: this bound is nonsense. actually figure out what it should be.
        let (sender, receiver) = crossbeam::channel::bounded(1);

        Self {
            context: AppContext::new(graphics, sender),
            receiver,
        }
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        self.context.spawn_window(spec, constructor);
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

#[derive(Clone)]
pub struct AppContext {
    sender: Sender<AppMessage>,
}

impl AppContext {
    fn new(_graphics: &GraphicsConfig, sender: Sender<AppMessage>) -> Self {
        // initialize renderer

        Self { sender }
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(crate::window::Window) -> W + Send + 'static,
    {
        spawn_window_thread(
            crate::application::AppContext {
                inner: self.clone(),
            },
            spec,
            constructor,
        );
    }
}
