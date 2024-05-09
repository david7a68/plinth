use std::{arch::x86_64, borrow::Cow};

use super::{
    platform_impl, {DpiScale, WindowExtent, WindowPoint},
};

use crate::{core::limit::Limit, time::FramesPerSecond};

#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[error("Windows cannot be created when the event loop is quitting.")]
    ExitingEventLoop,

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
    pub size: Option<WindowExtent>,
    pub min_size: Option<WindowExtent>,
    pub max_size: Option<WindowExtent>,
    pub position: Option<WindowPoint>,
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
    pub fn with_size(mut self, size: WindowExtent) -> Self {
        self.size = Some(size);
        self
    }

    #[must_use]
    pub fn with_min_size(mut self, min_size: WindowExtent) -> Self {
        self.min_size = Some(min_size);
        self
    }

    #[must_use]
    pub fn with_max_size(mut self, max_size: WindowExtent) -> Self {
        self.max_size = Some(max_size);
        self
    }

    #[must_use]
    pub fn with_position(mut self, position: WindowPoint) -> Self {
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

bitflags::bitflags! {
    pub(super) struct WindowFlags: u8 {
        const IS_VISIBLE = 0b0000_0001;
        const IS_RESIZABLE = 0b0000_0010;
        const HAS_FOCUS = 0b0000_0100;
        const HAS_POINTER = 0b0000_1000;
        const IS_RESIZING = 0b0001_0000;
        /// Keep this per-window, not per-event-loop because a different window
        /// might get a resize event while this one is still resizing. If that
        /// happens, we don't want the other window to get resize begin/end events.
        const IN_DRAG_RESIZE = 0b0010_0000;
    }
}

pub(super) struct WindowState {
    pub title: Cow<'static, str>,
    pub size: WindowExtent,
    pub min_size: WindowExtent,
    pub max_size: WindowExtent,
    pub position: WindowPoint,
    pub dpi: f32,
    pub flags: WindowFlags,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefreshRateRequest {
    pub min: FramesPerSecond,
    pub max: FramesPerSecond,
    pub preferred: FramesPerSecond,
}

pub struct Window<'a, User> {
    pub(super) user: &'a mut User,
    pub(super) state: &'a WindowState,
    pub(super) window: platform_impl::Window,
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
        self.user
    }

    #[must_use]
    pub fn data_mut(&mut self) -> &mut Data {
        self.user
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.state.title
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    #[must_use]
    pub fn size(&self) -> WindowExtent {
        self.state.size
    }

    pub fn set_size(&mut self, size: WindowExtent) {
        size.limit_assert();
        self.window.set_size(size);
    }

    #[must_use]
    pub fn min_size(&self) -> WindowExtent {
        self.state.min_size
    }

    pub fn set_min_size(&mut self, min_size: WindowExtent) {
        min_size.limit_assert();
        self.window.set_min_size(min_size);
    }

    #[must_use]
    pub fn max_size(&self) -> WindowExtent {
        self.state.max_size
    }

    pub fn set_max_size(&mut self, max_size: WindowExtent) {
        max_size.limit_assert();
        self.window.set_max_size(max_size);
    }

    #[must_use]
    pub fn position(&self) -> WindowPoint {
        self.state.position
    }

    pub fn set_position(&mut self, position: WindowPoint) {
        self.window.set_position(position);
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.state.flags.contains(WindowFlags::IS_VISIBLE)
    }

    pub fn show(&mut self) {
        self.window.show();
    }

    pub fn hide(&mut self) {
        self.window.hide();
    }

    #[must_use]
    pub fn is_resizable(&self) -> bool {
        self.state.flags.contains(WindowFlags::IS_RESIZABLE)
    }

    #[must_use]
    pub fn dpi_scale(&self) -> DpiScale {
        DpiScale::new(self.state.dpi)
    }

    #[must_use]
    pub fn has_focus(&self) -> bool {
        self.state.flags.contains(WindowFlags::HAS_FOCUS)
    }

    #[must_use]
    pub fn has_pointer(&self) -> bool {
        self.state.flags.contains(WindowFlags::HAS_POINTER)
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
        let (meta, user) = self.user;
        (
            meta,
            Window {
                user,
                state: self.state,
                window: self.window,
            },
        )
    }
}

impl<'a, Data> Window<'a, Option<Data>> {
    pub fn extract_option(self) -> Option<Window<'a, Data>> {
        if let Some(user) = self.user {
            Some(Window {
                user,
                state: self.state,
                window: self.window,
            })
        } else {
            None
        }
    }
}

impl<User> std::ops::Deref for Window<'_, User> {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        self.user
    }
}

impl<User> std::ops::DerefMut for Window<'_, User> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.user
    }
}
