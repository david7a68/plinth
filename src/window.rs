use crate::{
    animation::{AnimationFrequency, PresentTiming},
    application::AppContext,
    math::{Point, Scale, Size, Vec2},
    visuals::{Pixel, VisualTree},
};

#[cfg(target_os = "windows")]
use crate::backend::system::win32 as system;

pub const MAX_TITLE_LENGTH: usize = 255;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    X,
    Y,
    XY,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    pub title: String,
    pub size: Size<Window>,
    pub min_size: Option<Size<Window>>,
    pub max_size: Option<Size<Window>>,
    pub resizable: bool,
    pub visible: bool,
    pub animation_frequency: Option<AnimationFrequency>,
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
            animation_frequency: None,
        }
    }
}

pub struct Window {
    pub(crate) inner: system::Window,
}

impl Window {
    pub(crate) fn new(inner: system::Window) -> Self {
        Self { inner }
    }

    pub fn app(&self) -> &AppContext {
        self.inner.app()
    }

    pub fn close(&mut self) {
        self.inner.close();
    }

    pub fn begin_animation(&mut self, freq: Option<AnimationFrequency>) {
        self.inner.begin_animation(freq);
    }

    pub fn end_animation(&mut self) {
        self.inner.end_animation();
    }

    pub fn default_animation_frequency(&self) -> AnimationFrequency {
        self.inner.default_animation_frequency()
    }

    pub fn size(&self) -> Size<Window> {
        self.inner.size()
    }

    pub fn scale(&self) -> Scale<Window, Pixel> {
        self.inner.scale()
    }

    pub fn pointer_location(&self) -> Point<Window> {
        self.inner.pointer_location()
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.inner.set_visible(visible);
    }

    pub fn scene(&self) -> &VisualTree {
        todo!()
    }

    pub fn set_scene(&mut self, _scene: VisualTree) {
        todo!()
    }

    pub fn scene_mut<'a>(&'a mut self) -> &'a mut VisualTree {
        todo!()
    }
}

pub trait WindowEventHandler {
    fn event(&mut self, event: WindowEvent);
}

pub enum WindowEvent {
    CloseRequest,
    Destroy,
    Visible(bool),
    BeginResize,
    Resize(Size<Window>, Scale<Window, Pixel>),
    EndResize,
    Repaint(PresentTiming),
    PointerMove(Point<Window>, Vec2<Window>),
    Scroll(Axis, f32),
}
