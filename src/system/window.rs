use std::borrow::Cow;

use crate::frame::FramesPerSecond;

use super::{
    dpi::{DpiScale, WindowPoint, WindowSize},
    platform_impl,
    time::Instant,
};

#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[error(
        "The window title exceeds {} characters.",
        crate::limits::MAX_WINDOW_TITLE_LENGTH
    )]
    TitleTooLong,

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
    pub size: Option<WindowSize>,
    pub min_size: Option<WindowSize>,
    pub max_size: Option<WindowSize>,
    pub position: Option<WindowPoint>,
    pub is_visible: bool,
    pub is_resizable: bool,
}

impl WindowAttributes {
    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, size: WindowSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_min_size(mut self, min_size: WindowSize) -> Self {
        self.min_size = Some(min_size);
        self
    }

    pub fn with_max_size(mut self, max_size: WindowSize) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn with_position(mut self, position: WindowPoint) -> Self {
        self.position = Some(position);
        self
    }

    pub fn with_visibility(mut self, is_visible: bool) -> Self {
        self.is_visible = is_visible;
        self
    }

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

pub struct WindowWaker {
    pub(crate) waker: platform_impl::WindowWaker,
}

impl WindowWaker {
    /// Notifies the window that it should wake up.
    ///
    /// If the window still exists, this will cause the window event handler to
    /// call its `on_wake()` method.
    ///
    /// This function does nothing if the window has been destroyed. It is safe
    /// to do so, it just has no effect.
    pub fn wake(&self) {
        self.waker.wake()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaintInfo {
    pub target_present_time: Instant,
    pub target_refresh_rate: FramesPerSecond,
    pub prev_present_time: Instant,
    pub prev_target_present_time: Instant,
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

impl<'a, User> Window<'a, User> {
    pub fn waker(&self) -> WindowWaker {
        self.window.waker()
    }

    pub fn destroy(&mut self) {
        self.window.destroy();
    }

    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> windows::Win32::Foundation::HWND {
        self.window.hwnd()
    }

    pub fn user(&self) -> &User {
        self.window.data()
    }

    pub fn user_mut(&mut self) -> &mut User {
        self.window.data_mut()
    }

    pub fn map<Data2>(self, f: impl FnOnce(&'a mut User) -> &'a mut Data2) -> Window<'a, Data2> {
        self.window.map(f)
    }

    pub fn title(&self) -> &str {
        self.window.title()
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    pub fn size(&self) -> WindowSize {
        self.window.size()
    }

    pub fn set_size(&mut self, size: WindowSize) {
        self.window.set_size(size);
    }

    pub fn min_size(&self) -> WindowSize {
        self.window.min_size()
    }

    pub fn set_min_size(&mut self, min_size: WindowSize) {
        self.window.set_min_size(min_size);
    }

    pub fn max_size(&self) -> WindowSize {
        self.window.max_size()
    }

    pub fn set_max_size(&mut self, max_size: WindowSize) {
        self.window.set_max_size(max_size);
    }

    pub fn position(&self) -> WindowPoint {
        self.window.position()
    }

    pub fn set_position(&mut self, position: WindowPoint) {
        self.window.set_position(position);
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn show(&mut self) {
        self.window.show();
    }

    pub fn hide(&mut self) {
        self.window.hide();
    }

    pub fn is_resizable(&self) -> bool {
        self.window.is_resizable()
    }

    pub fn dpi_scale(&self) -> DpiScale {
        self.window.dpi_scale()
    }

    pub fn has_focus(&self) -> bool {
        self.window.has_focus()
    }

    pub fn has_pointer(&self) -> bool {
        self.window.has_pointer()
    }

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
    pub fn split(self) -> (&'a mut Meta, Window<'a, User>) {
        self.window.split()
    }
}
