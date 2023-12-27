use crate::{
    application::AppContext,
    graphics::{Canvas, FrameInfo, FramesPerSecond, RefreshRate},
    input::{Axis, ButtonState, MouseButton},
    math::{Point, Scale, Size, Vec2},
};

#[cfg(target_os = "windows")]
use crate::platform;

pub const MAX_TITLE_LENGTH: usize = 255;

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

pub trait WindowEventHandler {
    fn on_close_request(&mut self);

    fn on_destroy(&mut self) {}

    #[allow(unused_variables)]
    fn on_visible(&mut self, is_visible: bool) {}

    fn on_begin_resize(&mut self) {}

    #[allow(unused_variables)]
    fn on_resize(&mut self, size: Size<Window>, scale: Scale<Window, Window>) {}

    fn on_end_resize(&mut self) {}

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo);

    #[allow(unused_variables)]
    fn on_mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        location: Point<Window>,
    ) {
    }

    #[allow(unused_variables)]
    fn on_pointer_move(&mut self, location: Point<Window>, delta: Vec2<Window>) {}

    #[allow(unused_variables)]
    fn on_pointer_leave(&mut self) {}

    #[allow(unused_variables)]
    fn on_scroll(&mut self, axis: Axis, delta: f32) {}
}
