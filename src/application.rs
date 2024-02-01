use std::borrow::Cow;

use crate::{
    graphics::Image,
    io::{self, LocationId},
    window::{WindowError, WindowSpec},
    EventHandler, Window,
};

use crate::graphics::GraphicsConfig;

#[cfg(target_os = "windows")]
use crate::platform::win32 as platform;

pub struct Application {
    inner: platform::ApplicationImpl,
}

impl Application {
    #[must_use]
    pub fn new(graphics: &GraphicsConfig) -> Self {
        Self {
            inner: platform::ApplicationImpl::new(graphics),
        }
    }

    pub fn add_location(&mut self, location: impl io::Location) -> io::LocationId {
        self.inner.add_resource_location(location)
    }

    pub fn add_image_loader(
        &mut self,
        location: LocationId,
        loader: impl io::ImageLoader,
    ) -> Result<(), io::Error> {
        self.inner.add_image_loader(location, loader)
    }

    pub fn load_image(
        &mut self,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.inner.load_image(path)
    }

    pub fn load_image_from_location(
        &mut self,
        location: io::LocationId,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.inner.load_image_from_location(location, path)
    }

    pub fn spawn_window<W, F>(
        &mut self,
        spec: WindowSpec,
        constructor: F,
    ) -> Result<(), WindowError>
    where
        W: EventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
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
    pub fn add_location(&mut self, location: impl io::Location) -> io::LocationId {
        self.inner.add_resource_location(location)
    }

    pub fn add_image_loader(
        &mut self,
        location: LocationId,
        loader: impl io::ImageLoader,
    ) -> Result<(), io::Error> {
        self.inner.add_image_loader(location, loader)
    }

    pub fn load_image(
        &mut self,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.inner.load_image(path)
    }

    pub fn load_image_from_location(
        &mut self,
        location: io::LocationId,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.inner.load_image_from_location(location, path)
    }

    /// Spawns a new window on its own thread.
    ///
    /// The constructor is called on the new thread to initialize any per-window
    /// state once the window has been created, but before it is visible.
    pub fn spawn_window<W, F>(
        &mut self,
        spec: WindowSpec,
        constructor: F,
    ) -> Result<(), WindowError>
    where
        W: EventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.inner.spawn_window(spec, constructor)
    }
}
