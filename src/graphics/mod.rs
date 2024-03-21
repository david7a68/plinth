mod backend;
mod color;
mod image;
mod primitives;

use windows::Win32::Foundation::HWND;

use crate::{
    geometry::{Extent, Pixel, Rect, Scale, Wixel},
    system::power::PowerPreference,
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

pub use self::{
    color::Color,
    image::{Error as ImageError, Format, Image, Info as ImageInfo, Layout, PixelBuf},
    primitives::RoundRect,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    #[default]
    Auto,
    #[cfg(target_os = "windows")]
    Dx12,
}

#[derive(Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// The target refresh rate, if a frame rate has been set.
    pub target_frame_rate: Option<FramesPerSecond>,

    pub vblank_period: PresentPeriod,

    /// The estimated time that the next present will occur.
    pub next_present_time: PresentTime,

    /// The time that the last present occurred.
    pub prev_present_time: PresentTime,

    /// The time that the last present was scheduled to occur.
    pub prev_target_present_time: PresentTime,
}

pub(crate) struct Graphics {
    graphics: GraphicsImpl,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        match config.backend {
            Backend::Auto => {
                #[cfg(target_os = "windows")]
                {
                    Self {
                        graphics: GraphicsImpl::Dx12(backend::dx12::Graphics::new(config)),
                    }
                }
            }
            #[cfg(target_os = "windows")]
            Backend::Dx12 => Self {
                graphics: GraphicsImpl::Dx12(backend::dx12::Graphics::new(config)),
            },
        }
    }

    #[cfg(target_os = "windows")]
    pub fn create_window_context(&self, hwnd: HWND) -> WindowContext {
        let context = match &self.graphics {
            GraphicsImpl::Dx12(graphics) => ContextImpl::Dx12(graphics.create_context(hwnd)),
        };

        WindowContext { context }
    }

    /// Creates a new image.
    pub fn create_image(&self, info: &ImageInfo) -> Result<Image, ImageError> {
        todo!()
    }

    /// Uploads pixels for an image.
    ///
    /// The pixel buffer must be the same size as the image.
    pub fn upload_image(&self, image: Image, pixels: &PixelBuf) -> Result<(), ImageError> {
        todo!()
    }

    /// Removes an image from circulation.
    ///
    /// The image may continue to be used in the background until any pending
    /// drawing operations that use this image have completed.
    pub fn remove_image(&self, image: Image) {
        todo!()
    }

    /// Call to flush staging buffers.
    ///
    /// This does not block.
    pub fn upload_flush(&self) {
        todo!()
    }
}

pub(crate) struct WindowContext {
    context: ContextImpl,
}

impl WindowContext {
    pub fn resize(&mut self, size: Extent<Wixel>) {
        match &mut self.context {
            #[cfg(target_os = "windows")]
            ContextImpl::Dx12(context) => context.resize(size),
        }
    }

    pub fn change_dpi(&mut self, dpi: Scale<Wixel, Pixel>, size: Extent<Wixel>) {
        match &mut self.context {
            #[cfg(target_os = "windows")]
            ContextImpl::Dx12(context) => context.change_dpi(size, dpi),
        }
    }

    pub fn draw(&mut self, mut callback: impl FnMut(&mut Canvas, &FrameInfo)) {
        #[allow(clippy::infallible_destructuring_match /*, reason = "future backends coming"*/)]
        let context = match &mut self.context {
            #[cfg(target_os = "windows")]
            ContextImpl::Dx12(context) => context,
        };

        let (canvas, timing) = context.begin_draw();
        let mut canvas = Canvas {
            canvas: CanvasImpl::Dx12(canvas),
        };

        callback(&mut canvas, &timing);

        context.end_draw();
    }
}

pub struct Canvas<'a> {
    canvas: CanvasImpl<'a>,
}

impl Canvas<'_> {
    #[must_use]
    pub fn region(&self) -> Rect<Pixel> {
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
}

enum GraphicsImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Graphics),
}

enum ContextImpl {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Context),
}

enum CanvasImpl<'a> {
    #[cfg(target_os = "windows")]
    Dx12(backend::dx12::Canvas<'a>),
}
