mod backend;
mod color;
mod frame_statistics;
mod primitives;

use windows::Win32::Foundation::HWND;

use crate::{geometry::image, system::power::PowerPreference, WindowSize};

pub use self::color::*;
pub use self::frame_statistics::*;
pub use self::primitives::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    #[default]
    Auto,
    #[cfg(target_os = "windows")]
    Dx12,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
    pub backend: Backend,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::MaxPerformance,
            debug_mode: cfg!(debug_assertions),
            backend: Backend::Auto,
        }
    }
}

enum GraphicsImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Graphics),
}

pub struct Graphics {
    device: GraphicsImpl,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        todo!()
    }

    #[cfg(target_os = "windows")]
    pub fn create_window_context(&self, hwnd: HWND) -> WindowContext {
        let context = match &self.device {
            GraphicsImpl::Dx12(graphics) => ContextImpl::Dx12(graphics.create_context(hwnd)),
        };

        WindowContext { context }
    }
}

enum ContextImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Context),
}

pub struct WindowContext {
    context: ContextImpl,
}

impl WindowContext {
    pub fn resize(&mut self, size: WindowSize) {
        todo!()
    }

    pub fn begin_draw(&self) -> Canvas {
        todo!()
    }
}

pub struct Canvas<'a> {
    canvas: CanvasImpl<'a>,
}

impl Canvas<'_> {
    pub fn region(&self) -> image::Rect {
        match &self.canvas {
            #[cfg(target_os = "windows")]
            CanvasImpl::Dx12(canvas) => canvas.region(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        match &mut self.canvas {
            #[cfg(target_os = "windows")]
            CanvasImpl::Dx12(canvas) => canvas.clear(color),
        }
    }

    pub fn draw_rect(&mut self, rect: RoundRect) {
        match &mut self.canvas {
            #[cfg(target_os = "windows")]
            CanvasImpl::Dx12(canvas) => canvas.draw_rect(rect),
        }
    }

    pub fn finish(&mut self) {
        // todo
    }
}

enum CanvasImpl<'a> {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Canvas<'a>),
}
