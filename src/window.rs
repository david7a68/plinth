use crate::{
    animation::{AnimationFrequency, PresentTiming},
    application::AppContext,
    math::{Point, Scale, Size, Vec2},
    visuals::{Pixel, VisualTree},
};

#[cfg(target_os = "windows")]
use crate::system;

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
    inner: system::Window,
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

    pub fn scene_mut(&mut self) -> &mut VisualTree {
        todo!()
    }
}

pub trait WindowEventHandler {
    fn on_close_request(&mut self);

    fn on_destroy(&mut self) {}

    #[allow(unused_variables)]
    fn on_visible(&mut self, is_visible: bool) {}

    fn on_begin_resize(&mut self) {}

    #[allow(unused_variables)]
    fn on_resize(&mut self, size: Size<Window>, scale: Scale<Window, Pixel>) {}

    fn on_end_resize(&mut self) {}

    fn on_repaint(&mut self, timing: PresentTiming);

    #[allow(unused_variables)]
    fn on_pointer_move(&mut self, location: Point<Window>, delta: Vec2<Window>) {}

    #[allow(unused_variables)]
    fn on_scroll(&mut self, axis: Axis, delta: f32) {}
}
