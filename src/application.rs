use crate::window::{Window, WindowEventHandler, WindowSpec};

#[cfg(target_os = "windows")]
use crate::system;

pub use crate::graphics::{Config as GraphicsConfig, PowerPreference};

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

/// A reference-counted handle to application-wide state.
#[derive(Clone)]
#[repr(transparent)]
pub struct AppContext {
    pub(crate) inner: system::AppContext,
}

impl AppContext {
    /// Spawns a new window on its own thread.
    ///
    /// The constructor is called on the new thread to initialize any per-window
    /// state once the window has been created, but before it is visible.
    pub fn spawn_window<W, F>(&mut self, spec: WindowSpec, constructor: F)
    where
        W: WindowEventHandler + 'static,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.inner.spawn_window(spec, constructor);
    }
}

impl From<system::AppContext> for AppContext {
    fn from(inner: system::AppContext) -> Self {
        Self { inner }
    }
}
