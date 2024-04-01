use std::borrow::Cow;

use super::platform_impl;

use crate::{
    geometry::{Extent, Pixel, Point, Scale, Wixel},
    limits,
    time::FramesPerSecond,
};

#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[error("The maximum number of windows is open. Destroy one before creating another.")]
    TooManyWindows,

    #[error("A platform error occurred.")]
    Platform(#[from] platform_impl::WindowError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PaintReason {
    /// The window is being painted because the user requested it with `WindowMut::request_repaint()`.
    Requested,
    /// The window is being painted because it is animating.
    Animating,
    /// The window is being painted because the platform has determined that it
    /// needs to be repainted.
    ///
    /// THIS MUST BE DONE SYNCRHONOUSLY.
    Commanded,
}

pub struct WindowAttributes {
    pub title: Cow<'static, str>,
    pub size: Option<Extent<Wixel>>,
    pub min_size: Option<Extent<Wixel>>,
    pub max_size: Option<Extent<Wixel>>,
    pub position: Option<Point<Wixel>>,
    pub is_visible: bool,
    pub is_resizable: bool,
}

impl WindowAttributes {
    #[must_use]
    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn with_size(mut self, size: Extent<Wixel>) -> Self {
        self.size = Some(size);
        self
    }

    #[must_use]
    pub fn with_min_size(mut self, min_size: Extent<Wixel>) -> Self {
        self.min_size = Some(min_size);
        self
    }

    #[must_use]
    pub fn with_max_size(mut self, max_size: Extent<Wixel>) -> Self {
        self.max_size = Some(max_size);
        self
    }

    #[must_use]
    pub fn with_position(mut self, position: Point<Wixel>) -> Self {
        self.position = Some(position);
        self
    }

    #[must_use]
    pub fn with_visibility(mut self, is_visible: bool) -> Self {
        self.is_visible = is_visible;
        self
    }

    #[must_use]
    pub fn with_resizability(mut self, is_resizable: bool) -> Self {
        self.is_resizable = is_resizable;
        self
    }
}

impl Default for WindowAttributes {
    fn default() -> Self {
        WindowAttributes {
            title: Cow::Borrowed(""),
            size: None,
            min_size: None,
            max_size: None,
            position: None,
            is_visible: true,
            is_resizable: true,
        }
    }
}

pub struct Waker {
    pub(crate) waker: platform_impl::Waker,
}

impl Waker {
    /// Notifies the window that it should wake up.
    ///
    /// If the window still exists, this will cause the window event handler to
    /// call its `on_wake()` method.
    ///
    /// This function does nothing if the window has been destroyed. It is safe
    /// to do so, it just has no effect.
    pub fn wake(&self) {
        self.waker.wake();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefreshRateRequest {
    pub min: FramesPerSecond,
    pub max: FramesPerSecond,
    pub preferred: FramesPerSecond,
}

pub struct Window<'a, User> {
    pub(crate) window: platform_impl::Window<'a, User>,
}

impl<'a, Data> Window<'a, Data> {
    #[must_use]
    pub fn waker(&self) -> Waker {
        self.window.waker()
    }

    pub fn destroy(&mut self) {
        self.window.destroy();
    }

    #[must_use]
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> windows::Win32::Foundation::HWND {
        self.window.hwnd()
    }

    #[must_use]
    pub fn data(&self) -> &Data {
        self.window.data()
    }

    #[must_use]
    pub fn data_mut(&mut self) -> &mut Data {
        self.window.data_mut()
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.window.title()
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    #[must_use]
    pub fn size(&self) -> Extent<Wixel> {
        self.window.size()
    }

    pub fn set_size(&mut self, size: Extent<Wixel>) {
        limits::SYS_WINDOW_EXTENT.check(size);
        self.window.set_size(size);
    }

    #[must_use]
    pub fn min_size(&self) -> Extent<Wixel> {
        self.window.min_size()
    }

    pub fn set_min_size(&mut self, min_size: Extent<Wixel>) {
        limits::SYS_WINDOW_EXTENT.check(min_size);
        self.window.set_min_size(min_size);
    }

    #[must_use]
    pub fn max_size(&self) -> Extent<Wixel> {
        self.window.max_size()
    }

    pub fn set_max_size(&mut self, max_size: Extent<Wixel>) {
        limits::SYS_WINDOW_EXTENT.check(max_size);
        self.window.set_max_size(max_size);
    }

    #[must_use]
    pub fn position(&self) -> Point<Wixel> {
        self.window.position()
    }

    pub fn set_position(&mut self, position: Point<Wixel>) {
        self.window.set_position(position);
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn show(&mut self) {
        self.window.show();
    }

    pub fn hide(&mut self) {
        self.window.hide();
    }

    #[must_use]
    pub fn is_resizable(&self) -> bool {
        self.window.is_resizable()
    }

    #[must_use]
    pub fn dpi_scale(&self) -> Scale<Wixel, Pixel> {
        self.window.dpi_scale()
    }

    #[must_use]
    pub fn has_focus(&self) -> bool {
        self.window.has_focus()
    }

    #[must_use]
    pub fn has_pointer(&self) -> bool {
        self.window.has_pointer()
    }

    #[must_use]
    pub fn frame_rate(&self) -> FramesPerSecond {
        self.window.frame_rate()
    }

    pub fn request_refresh_rate(&mut self, rate: RefreshRateRequest, after_next_present: bool) {
        self.window.request_refresh_rate(rate, after_next_present);
    }

    pub fn request_repaint(&mut self) {
        self.window.request_repaint();
    }
}

impl<'a, Meta, User> Window<'a, (Meta, User)> {
    pub(crate) fn split(self) -> (&'a mut Meta, Window<'a, User>) {
        self.window.split()
    }
}
