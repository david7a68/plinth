use crate::window::{Window, WindowEventHandler, WindowSpec};

#[cfg(target_os = "windows")]
use crate::backend::system::win32 as system;

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
}

#[derive(Clone)]
pub struct Application {
    inner: system::Application,
}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        Self {
            inner: system::Application::new(graphics),
        }
    }

    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.inner.spawn_window(spec, constructor);
    }

    /// Runs the application is finished.
    ///
    /// This returns when all windows are closed. This may only be called once.
    pub fn run(&mut self) {
        self.inner.run();
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub(crate) inner: system::AppContext,
}

impl AppContext {
    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.inner.spawn_window(spec, constructor);
    }
}
