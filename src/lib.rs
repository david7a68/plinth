#![allow(dead_code, unused_variables)]

pub mod color;
pub mod math;

use std::time::Instant;

pub use color::{Color, ColorSpace, Srgb};
pub use math::{
    CoordinateUnit, Pixel, PixelsPerSecond, Point2D, Rect2D, Scale2D, Size2D, Translate2D,
};

pub struct DevicePixel;

impl CoordinateUnit for DevicePixel {}

pub enum Axis {
    X,
    Y,
    XY,
}

#[derive(Clone, Copy)]
pub struct WindowSize {
    pub physical: Size2D<DevicePixel>,
    pub logical: Size2D<Pixel>,
    pub scale_factor: Scale2D<DevicePixel, Pixel>,
}

pub struct WindowSpec {
    pub title: String,
    pub size: Size2D<Pixel>,
    pub min_size: Option<Size2D<Pixel>>,
    pub max_size: Option<Size2D<Pixel>>,
    pub resizable: bool,
    pub visible: bool,
    pub animation_frequency: Option<AnimationFrequency>,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: String::new(),
            size: Size2D::new(800.0, 600.0),
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

#[derive(Clone, Copy)]
pub struct SceneNodeId {
    index: u32,
    generation: u32,
}

pub struct Scene {
    nodes: Vec<SceneNode>,
    free_list: Vec<SceneNodeId>,
}

impl Scene {
    pub fn new() -> Self {
        todo!()
    }

    pub fn set_root(&mut self, id: SceneNodeId) {
        todo!()
    }

    pub fn new_root(&mut self, node: impl Into<SceneNode>) -> SceneNodeId {
        todo!()
    }

    pub fn root_id(&self) -> Option<SceneNodeId> {
        todo!()
    }

    pub fn root(&self) -> Option<&SceneNode> {
        todo!()
    }

    pub fn root_mut(&mut self) -> Option<&mut SceneNode> {
        todo!()
    }

    pub fn node(&self, id: SceneNodeId) -> Option<&SceneNode> {
        todo!()
    }

    pub fn new_node(&mut self, node: impl Into<SceneNode>) -> SceneNodeId {
        todo!()
    }

    pub fn destroy_node(&mut self, id: SceneNodeId) {
        todo!()
    }

    pub fn add_child(&mut self, parent: SceneNodeId, child: SceneNodeId) {
        todo!()
    }

    pub fn new_child(&mut self, parent: SceneNodeId, child: impl Into<SceneNode>) -> SceneNodeId {
        todo!()
    }

    pub fn remove_child(&mut self, parent: SceneNodeId, child: SceneNodeId) {
        todo!()
    }
}

pub enum SceneNode {
    Unused,
    Background(Color<Srgb>),
    Canvas(Canvas),
    Image(()),
}

pub struct Canvas {
    dummy: u64,
}

impl Canvas {
    pub fn new() -> Self {
        todo!()
    }

    pub fn clear(&mut self, color: Color<Srgb>) {
        todo!()
    }

    pub fn fill(&mut self, rect: Rect2D<Pixel>, color: Color<Srgb>) {
        todo!()
    }
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
