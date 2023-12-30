use crate::window::{Window, WindowError, WindowEventHandler, WindowSpec};

use crate::graphics::GraphicsConfig;
#[cfg(target_os = "windows")]
use crate::platform;

pub struct Application {
    inner: platform::ApplicationImpl,
}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        Self {
            inner: platform::ApplicationImpl::new(graphics),
        }
    }

    pub fn spawn_window(
        &mut self,
        spec: WindowSpec,
        constructor: &'static (dyn Fn(Window) -> Box<dyn WindowEventHandler> + Send),
    ) -> Result<(), WindowError> {
        self.inner.spawn_window(spec, constructor)
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
    pub(crate) inner: platform::AppContextImpl,
}

impl AppContext {
    /// Spawns a new window on its own thread.
    ///
    /// The constructor is called on the new thread to initialize any per-window
    /// state once the window has been created, but before it is visible.
    pub fn spawn_window(
        &mut self,
        spec: WindowSpec,
        constructor: &'static (dyn Fn(Window) -> Box<dyn WindowEventHandler> + Send),
    ) -> Result<(), WindowError> {
        self.inner.spawn_window(spec, constructor)
    }
}

impl From<platform::AppContextImpl> for AppContext {
    fn from(inner: platform::AppContextImpl) -> Self {
        Self { inner }
    }
}
