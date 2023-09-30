#![allow(dead_code, unused_variables)]

pub mod color;
pub mod math;
pub mod scene;

use std::{path::Path, time::Instant};

use color::{Color, Srgb};
use math::{Pixels, Point, Rect, Scale, Size, Vec2};
use scene::{FromVisual, Visual, VisualTree};

pub struct DevicePixels {}

pub enum Axis {
    X,
    Y,
    XY,
}

pub struct WindowSpec {
    pub title: String,
    pub size: Size<Pixels>,
    pub min_size: Option<Size<Pixels>>,
    pub max_size: Option<Size<Pixels>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowError {
    AlreadyClosed,
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

    pub fn close(&mut self) -> Result<(), WindowError> {
        todo!()
    }

    pub fn begin_animation(&mut self, freq: Option<AnimationFrequency>) -> Result<(), WindowError> {
        todo!()
    }

    pub fn end_animation(&mut self) -> Result<(), WindowError> {
        todo!()
    }

    pub fn default_animation_frequency(&self) -> Result<AnimationFrequency, WindowError> {
        todo!()
    }

    pub fn size(&self) -> Result<Size<Pixels>, WindowError> {
        todo!()
    }

    pub fn scale(&self) -> Result<Scale<DevicePixels, Pixels>, WindowError> {
        todo!()
    }

    pub fn pointer_location(&self) -> Result<Point<Pixels>, WindowError> {
        todo!()
    }

    pub fn scene(&self) -> Result<&VisualTree, WindowError> {
        todo!()
    }

    pub fn set_scene(&mut self, scene: VisualTree) -> Result<(), WindowError> {
        todo!()
    }

    pub fn scene_mut<'a>(&'a mut self) -> Result<&'a mut VisualTree, WindowError> {
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
    Resize(Size<Pixels>, Scale<Pixels, DevicePixels>),
    EndResize,
    Repaint(PresentTiming),
    PointerMove(Point<Pixels>, Vec2<Pixels>),
    Scroll(Axis, f32),
}

pub struct PresentTiming {
    pub next_frame: Instant,
    pub last_frame: Instant,
}

pub struct AnimationFrequency {
    /// The minimum rate at which the window would like to receive repaint events.
    pub min_fps: Option<f32>,
    /// The maximum rate at which the window would like to receive repaint events.
    pub max_fps: Option<f32>,
    /// The optimal rate at which the window would like to receive repaint events.
    pub optimal_fps: f32,
}

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
}

pub struct Application {}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Self {
        todo!()
    }

    pub fn create_window<W, F>(
        &mut self,
        spec: &WindowSpec,
        constructor: F,
    ) -> Result<(), WindowError>
    where
        W: WindowEventHandler + 'static,
        F: FnMut(Window) -> W + 'static,
    {
        todo!()
    }

    /// Runs the event loop until all open windows are closed.
    pub fn run(&mut self) {
        todo!()
    }
}

impl Clone for Application {
    fn clone(&self) -> Self {
        todo!()
    }
}

pub struct Canvas {}

impl Canvas {
    pub fn new() -> Self {
        todo!()
    }

    pub fn clear(&mut self, color: Color<Srgb>) {
        todo!()
    }

    pub fn fill(&mut self, rect: Rect<Pixels>, color: Color<Srgb>) {
        todo!()
    }
}

impl From<Canvas> for Visual {
    fn from(canvas: Canvas) -> Self {
        Self::Canvas(canvas)
    }
}

impl FromVisual for Canvas {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Canvas(canvas) => Some(canvas),
            _ => None,
        }
    }
}

pub struct ImageView {}

impl From<ImageView> for Pixels {
    fn from(image_view: ImageView) -> Self {
        Pixels {}
    }
}

pub struct Image {}

impl Image {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ()> {
        todo!()
    }

    pub fn size(&self) -> Size<Pixels> {
        todo!()
    }
}

impl From<Image> for Visual {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl FromVisual for Image {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Image(image) => Some(image),
            _ => None,
        }
    }
}

pub struct Text {}

impl Text {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Text> for Visual {
    fn from(text: Text) -> Self {
        Self::Text(text)
    }
}

impl FromVisual for Text {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Text(text) => Some(text),
            _ => None,
        }
    }
}

pub struct Panel {}

impl Panel {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Panel> for Visual {
    fn from(panel: Panel) -> Self {
        Self::Panel(panel)
    }
}

impl FromVisual for Panel {
    fn from_node(node: Visual) -> Option<Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }

    fn from_ref(node: &Visual) -> Option<&Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }

    fn from_mut(node: &mut Visual) -> Option<&mut Self> {
        match node {
            Visual::Panel(panel) => Some(panel),
            _ => None,
        }
    }
}
