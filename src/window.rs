use crate::{
    application::AppContext,
    graphics::{Canvas, FrameInfo, FramesPerSecond, RefreshRate},
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

#[derive(Clone, Copy, Debug)]
pub enum Input {
    MouseButton(MouseButton, ButtonState, WindowPoint),
    PointerMove(WindowPoint),
    PointerLeave,
    Scroll(Axis, f32),
}

#[derive(Clone, Copy, Debug)]
pub enum WindowEvent {
    CloseRequest,
    Visible(bool),
    BeginResize,
    Resize(WindowSize),
    EndResize,
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

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    pub size: Size<Window>,
    pub min_size: Option<Size<Window>>,
    pub max_size: Option<Size<Window>>,
    pub resizable: bool,
    pub visible: bool,
    pub refresh_rate: Option<FramesPerSecond>,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: String::new(),
            size: Size::new(800.0, 600.0),
            min_size: None,
            max_size: None,
            resizable: true,
            visible: true,
            refresh_rate: None,
        }
    }
}

pub struct Window {
    inner: platform::WindowImpl,
}

impl Window {
    pub(crate) fn new(inner: platform::WindowImpl) -> Self {
        Self { inner }
    }

    pub fn app(&self) -> &AppContext {
        self.inner.app()
    }

    pub fn close(&mut self) {
        self.inner.close();
    }

    pub fn set_animation_frequency(&mut self, freq: FramesPerSecond) {
        self.inner.set_animation_frequency(freq);
    }

    pub fn refresh_rate(&self) -> RefreshRate {
        self.inner.refresh_rate()
    }

    pub fn size(&self) -> Size<Window> {
        self.inner.size()
    }

    /// The HiDPI scale factor.
    pub fn scale(&self) -> Scale<Window, Window> {
        self.inner.scale()
    }

    pub fn pointer_location(&self) -> Option<Point<Window>> {
        self.inner.pointer_location()
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.inner.set_visible(visible);
    }
}

pub type WindowEventHandlerConstructor =
    dyn (Fn(Window) -> Box<dyn WindowEventHandler>) + Send + Sync;

pub trait WindowEventHandler: Send {
    fn on_event(&mut self, event: WindowEvent);

    fn on_input(&mut self, input: Input);

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo);
}

impl<W: WindowEventHandler> WindowEventHandler for Box<W> {
    fn on_event(&mut self, event: WindowEvent) {
        self.as_mut().on_event(event);
    }

    fn on_input(&mut self, input: Input) {
        self.as_mut().on_input(input);
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        self.as_mut().on_repaint(canvas, timing);
    }
}
