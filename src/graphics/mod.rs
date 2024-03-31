mod backend;
mod color;
mod image;
pub mod limits;
mod primitives;

use windows::Win32::Foundation::HWND;

use crate::{
    geometry::{Extent, Pixel, Point},
    graphics::image::PackedKey,
    system::power::PowerPreference,
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use self::backend::{
    texture_atlas::{CachedTextureId, TextureCache},
    Device, Uploader,
};

pub use self::{
    backend::draw_list::Canvas,
    backend::WindowContext,
    color::Color,
    image::{
        Command as PathCommand, CommandIter as PathCommandIter, Error as ImageError, Format, Image,
        Info as ImageInfo, Layout, PathIter, PathRef, RasterBuf, VectorBuf, Verb as PathVerb,
    },
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
    device: Device,
    uploader: Uploader,
    textures: TextureCache,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = Device::new(config);

        let mut uploader = device.create_uploader();

        let textures = TextureCache::new(
            Extent::new(1, 1),
            Layout::Rgba8,
            Format::Linear,
            |extent, layout, format| device.create_texture(extent, layout, format),
        );

        let (_, white_pixel) = textures.default();

        uploader.upload_image(
            white_pixel,
            &RasterBuf::new(
                ImageInfo {
                    extent: Extent::new(1, 1),
                    layout: Layout::Rgba8,
                    format: Format::Linear,
                },
                &[0xFF, 0xFF, 0xFF, 0xFF],
            ),
            Point::new(0, 0),
        );

        Self {
            device,
            uploader,
            textures,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn create_window_context(&self, hwnd: HWND) -> WindowContext {
        self.device.create_context(hwnd)
    }

    pub fn create_raster_image(&mut self, info: ImageInfo) -> Result<Image, ImageError> {
        let (_, texture_id) = self.textures.insert_rect(
            info.extent,
            info.layout,
            info.format,
            |extent, layout, format| self.device.create_texture(extent, layout, format),
        );

        let image = Image {
            info: info.pack(),
            key: PackedKey::new()
                .with_index(texture_id.index())
                .with_epoch(texture_id.epoch()),
        };

        Ok(image)
    }

    pub fn create_vector_image(
        &mut self,
        buf: VectorBuf,
        initial_size: Option<Extent<Pixel>>,
    ) -> Result<Image, ImageError> {
        // calculate broad bounds for the image, use that as exent

        if let Some(size) = initial_size {
            // rasterize immediately

            // self.textures.insert_rect(...);

            todo!()
        }

        // layout and format are the same as the input

        todo!()
    }

    /// Uploads pixels for an image.
    ///
    /// The pixel buffer must be the same size as the image.
    pub fn upload_raster_image(
        &mut self,
        image: Image,
        pixels: &RasterBuf,
    ) -> Result<(), ImageError> {
        let cache_id = CachedTextureId::new(image.key.index(), image.key.epoch());
        let (texture, rect) = self.textures.get_rect(cache_id);

        self.uploader.upload_image(texture, pixels, rect.origin);

        Ok(())
    }

    /// Removes an image from circulation.
    ///
    /// The image may continue to be used in the background until any pending
    /// drawing operations that use this image have completed.
    pub fn delete_image(&mut self, image: Image) {
        let _ = image;
        todo!()
    }

    /// Call to flush staging buffers.
    ///
    /// This does not block.
    pub fn flush_upload_buffer(&mut self) {
        self.uploader.flush_upload_buffer();
    }

    pub fn draw(&self, window: &mut WindowContext, callback: impl FnMut(&mut Canvas, &FrameInfo)) {
        window.draw(&self.textures, callback);
    }
}
