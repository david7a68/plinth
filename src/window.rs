use crate::{
    application::AppContext,
    frame::{RedrawRequest, RefreshRate},
    graphics::{Canvas, FrameInfo},
    math::{Point, Scale, Size},
};

#[cfg(target_os = "windows")]
use crate::platform;
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    X,
    Y,
    XY,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Aux1,
    Aux2,
    Other(u8),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowError {
    // todo: implement error trait
    TooManyWindows,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowSize {
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowPoint {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Debug)]
pub struct WindowSpec {
    pub title: String,
    pub size: WindowSize,
    pub min_size: Option<WindowSize>,
    pub max_size: Option<WindowSize>,
    pub resizable: bool,
    pub visible: bool,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: String::new(),
            size: WindowSize {
                width: 800,
                height: 600,
                dpi: 96,
            },
            min_size: None,
            max_size: None,
            resizable: true,
            visible: true,
        }
    }
}

pub struct Window {
    pub(crate) inner: platform::WindowImpl,
}

impl Window {
    pub(crate) fn new(inner: platform::WindowImpl) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn app(&self) -> &AppContext {
        self.inner.app()
    }

    pub fn close(&mut self) {
        self.inner.close();
    }

    pub fn request_redraw(&mut self, request: RedrawRequest) {
        // mut to suggest that this has side effects
        self.inner.request_redraw(request);
    }

    #[must_use]
    pub fn refresh_rate(&self) -> RefreshRate {
        self.inner.refresh_rate()
    }

    #[must_use]
    pub fn size(&self) -> Size<Window> {
        self.inner.size()
    }

    /// The `HiDPI` scale factor.
    #[must_use]
    pub fn scale(&self) -> Scale<Window, Window> {
        self.inner.scale()
    }

    #[must_use]
    pub fn pointer_location(&self) -> Option<Point<Window>> {
        self.inner.pointer_location()
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.inner.set_visible(visible);
    }
}

pub trait EventHandler: Send + 'static {
    fn on_close_request(&mut self);

    fn on_visible(&mut self, visible: bool);

    fn on_begin_resize(&mut self);

    fn on_resize(&mut self, size: WindowSize);

    fn on_end_resize(&mut self);

    fn on_mouse_button(&mut self, button: MouseButton, state: ButtonState, location: WindowPoint);

    fn on_pointer_move(&mut self, location: WindowPoint);

    fn on_pointer_leave(&mut self);

    fn on_scroll(&mut self, axis: Axis, delta: f32);

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo);
}
