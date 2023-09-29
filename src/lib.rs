#![allow(dead_code, unused_variables)]

pub mod color;
#[macro_use]
pub mod math;
pub mod scene;

use std::{path::Path, time::Instant};

use color::{Color, Srgb};
use math::{Pixels, Rect, Scale, Size};
use scene::{Scene, SceneNode};

pub struct DevicePixels {}

pub enum Axis {
    X,
    Y,
    XY,
}

#[derive(Clone, Copy)]
pub struct WindowSize {
    pub physical: Size<DevicePixels>,
    pub logical: Size<Pixels>,
    pub scale_factor: Scale<DevicePixels, Pixels>,
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

    pub fn size(&self) -> Result<WindowSize, WindowError> {
        todo!()
    }

    pub fn scene(&self) -> Result<&Scene, WindowError> {
        todo!()
    }

    pub fn set_scene(&mut self, scene: Scene) -> Result<(), WindowError> {
        todo!()
    }

    pub fn scene_mut<'a>(&'a mut self) -> Result<&'a mut Scene, WindowError> {
        todo!()
    }

    /// Synchronizes the scene graph with the window's rendered appearance.
    pub fn sync_scene(&mut self) -> Result<(), WindowError> {
        todo!()
    }
}

pub trait WindowEventHandler {
    fn event(&mut self, event: WindowEvent);
}

pub enum WindowEvent {
    Create(Window),
    CloseRequest,
    Destroy,
    Visible(bool),
    BeginResize(Axis),
    Resize(WindowSize),
    EndResize,
    Repaint(PresentTiming),
    Scroll(Axis, f32),
}

pub fn new_window<W: WindowEventHandler + 'static, F: FnMut(Window) -> W + 'static>(
    spec: WindowSpec,
    handler: F,
) -> Window {
    todo!()
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

    /// Runs the event loop until all open windows are closed.
    pub fn run(&mut self) {
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

impl From<Canvas> for SceneNode {
    fn from(canvas: Canvas) -> Self {
        Self::Canvas(canvas)
    }
}

impl TryFrom<SceneNode> for Canvas {
    type Error = ();

    fn try_from(node: SceneNode) -> Result<Self, Self::Error> {
        match node {
            SceneNode::Canvas(canvas) => Ok(canvas),
            _ => Err(()),
        }
    }
}

pub struct Image {}

impl Image {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ()> {
        todo!()
    }
}

impl From<Image> for SceneNode {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl TryFrom<SceneNode> for Image {
    type Error = ();

    fn try_from(node: SceneNode) -> Result<Self, Self::Error> {
        match node {
            SceneNode::Image(image) => Ok(image),
            _ => Err(()),
        }
    }
}

pub struct Text {}

impl Text {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Text> for SceneNode {
    fn from(text: Text) -> Self {
        Self::Text(text)
    }
}

impl TryFrom<SceneNode> for Text {
    type Error = ();

    fn try_from(node: SceneNode) -> Result<Self, Self::Error> {
        match node {
            SceneNode::Text(text) => Ok(text),
            _ => Err(()),
        }
    }
}

pub struct Panel {}

impl Panel {
    pub fn new() -> Self {
        todo!()
    }
}

impl From<Panel> for SceneNode {
    fn from(panel: Panel) -> Self {
        Self::Panel(panel)
    }
}

impl TryFrom<SceneNode> for Panel {
    type Error = ();

    fn try_from(node: SceneNode) -> Result<Self, Self::Error> {
        match node {
            SceneNode::Panel(panel) => Ok(panel),
            _ => Err(()),
        }
    }
}
