mod backend;
mod color;
mod draw_list;
mod image;
pub(crate) mod limits;
mod primitives;
mod texture_atlas;

use windows::Win32::Foundation::HWND;

use crate::{
    geometry::{Extent, Pixel, Point, Rect, Scale, Texel},
    graphics::image::PackedKey,
    system::PowerPreference,
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use self::{
    backend::Device,
    texture_atlas::{CachedTextureId, TextureCache},
};

pub(crate) use self::backend::Swapchain;

pub use self::{
    backend::RenderTarget,
    color::Color,
    draw_list::{Canvas, DrawList},
    image::{Error as ImageError, Format, Image, Info as ImageInfo, Layout, RasterBuf},
    primitives::RoundRect,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    #[default]
    Auto,
    Null,
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

pub struct Graphics {
    device: Device,
    textures: TextureCache,
}

impl Graphics {
    pub(crate) fn new(config: &GraphicsConfig) -> Self {
        let device = Device::new(config);

        let textures = TextureCache::new(
            Extent::new(1, 1),
            Layout::Rgba8,
            Format::Linear,
            |extent, layout, format| device.create_texture(extent, layout, format),
        );

        let (_, white_pixel) = textures.default();

        device.copy_raster_to_texture(
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

        device.flush_upload_buffer();

        Self { device, textures }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn create_swapchain(&self, hwnd: HWND) -> Swapchain {
        self.device.create_swapchain(hwnd)
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

        self.device
            .copy_raster_to_texture(texture, pixels, rect.origin);

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
        self.device.flush_upload_buffer();
    }

    pub fn create_canvas<'a>(
        &'a self,
        target: &'a RenderTarget,
        draw_list: &'a mut DrawList,
        scale: Scale<Texel, Pixel>,
    ) -> Canvas<'a> {
        let rect = Rect::new(Point::ZERO, target.extent().scale_to(scale));
        Canvas::new(&self.textures, draw_list, rect)
    }

    pub fn draw(&self, draw_list: &DrawList, target: &mut RenderTarget) {
        self.device.draw(draw_list, target);
    }
}
