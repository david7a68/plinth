use crate::{
    animation::{AnimationFrequency, PresentTiming},
    application::Application,
    math::{Point, Scale, Size, Vec2},
    visuals::{Pixel, VisualTree},
};

pub enum Axis {
    X,
    Y,
    XY,
}

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
    // todo
    #[allow(dead_code)]
    dummy: u64,
}

impl Window {
    pub fn app(&self) -> &Application {
        todo!()
    }

    pub fn close(&mut self) {
        todo!()
    }

    pub fn begin_animation(&mut self, freq: Option<AnimationFrequency>) {
        todo!()
    }

    pub fn end_animation(&mut self) {
        todo!()
    }

    pub fn default_animation_frequency(&self) -> AnimationFrequency {
        todo!()
    }

    pub fn size(&self) -> Size<Window> {
        todo!()
    }

    pub fn scale(&self) -> Scale<Window, Pixel> {
        todo!()
    }

    pub fn pointer_location(&self) -> Point<Window> {
        todo!()
    }

    pub fn scene(&self) -> &VisualTree {
        todo!()
    }

    pub fn set_scene(&mut self, scene: VisualTree) {
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
    BeginResize(Axis),
    Resize(Size<Window>, Scale<Window, Pixel>),
    EndResize,
    Repaint(PresentTiming),
    PointerMove(Point<Window>, Vec2<Window>),
    Scroll(Axis, f32),
}
