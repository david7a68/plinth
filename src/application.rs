use crate::window::{Window, WindowEventHandler, WindowSpec};

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
}

pub struct Application {}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        todo!()
    }

    pub fn spawn_window<W, F>(&mut self, spec: &WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(Window) -> W + 'static,
    {
        todo!()
    }

    /// Runs the event loop until all open windows are closed.
    pub fn run(&mut self) {
        todo!()
    }
}

impl Clone for Application {
    fn clone(&self) -> Self {
        todo!()
    }
}
