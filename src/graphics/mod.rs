mod color;
mod draw_list;
mod gl;
mod image;
pub(crate) mod limits;
mod primitives;
mod text;
mod texture_atlas;

use windows::Win32::Foundation::HWND;

use crate::{
    core::arena::Arena,
    geometry::{new_extent, new_point, new_rect, Extent, Point, Rect},
    graphics::image::PackedKey,
    system::{DpiScale, PowerPreference, WindowExtent},
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use self::{
    gl::Device,
    limits::{
        GFX_ATLAS_EXTENT_MAX, GFX_ATLAS_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, GFX_IMAGE_EXTENT_MIN,
    },
    text::TextEngine,
    texture_atlas::{CachedTextureId, TextureCache},
};

pub(crate) use self::gl::Swapchain;

pub use self::{
    color::Color,
    draw_list::{Canvas, DrawList},
    gl::RenderTarget,
    image::{Error as ImageError, Format, Image, Info as ImageInfo, Layout, RasterBuf},
    primitives::RoundRect,
    text::{
        Error as TextError, FontOptions, Pt, Shape as FontShape, TextBox, TextWrapMode,
        Weight as FontWeight,
    },
};

new_point!(UvPoint(u, v), f32, 0.0, { limit: 0.0, 1.0, "UV point out of limits" });
new_extent!(UvExtent, f32, 0.0, { limit: 0.0, 1.0, "UV extent out of limits" });
new_rect!(UvRect, f32, UvPoint, UvExtent);

impl UvRect {
    pub fn to_uvwh(&self) -> [f32; 4] {
        [
            self.origin.u,
            self.origin.v,
            self.extent.width,
            self.extent.height,
        ]
    }
}

new_point!(ImagePoint(x, y), u16, 0, { limit: GFX_IMAGE_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, "Image point out of limits" }, Eq);
new_extent!(
    #[derive(Hash)]
    ImageExtent, u16, 0, { limit: GFX_IMAGE_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, "Image extent out of limits" }, Eq);

impl ImageExtent {
    pub const fn limit_assert(extent: Self) {
        assert!(extent.width >= GFX_IMAGE_EXTENT_MIN);
        assert!(extent.height >= GFX_IMAGE_EXTENT_MIN);
        assert!(extent.width <= GFX_IMAGE_EXTENT_MAX);
        assert!(extent.height <= GFX_IMAGE_EXTENT_MAX);
    }
}

impl From<ImageExtent> for TextureExtent {
    fn from(extent: ImageExtent) -> Self {
        Self {
            width: extent.width,
            height: extent.height,
        }
    }
}

impl From<WindowExtent> for TextureExtent {
    fn from(extent: WindowExtent) -> Self {
        Self {
            width: extent.width as u16,
            height: extent.height as u16,
        }
    }
}

new_rect!(ImageRect, u16, ImagePoint, ImageExtent);

new_point!(TexturePoint(x, y), u16, 0, { limit: GFX_ATLAS_EXTENT_MIN, GFX_ATLAS_EXTENT_MAX, "Texture point out of limits" }, Eq);
new_extent!(TextureExtent, u16, 0, { limit: GFX_ATLAS_EXTENT_MIN, GFX_ATLAS_EXTENT_MAX, "Texture extent out of limits" }, Eq);
new_rect!(TextureRect, u16, TexturePoint, TextureExtent);

impl TextureRect {
    pub fn uv_in(&self, texture: TextureExtent) -> UvRect {
        debug_assert!(self.extent.width <= texture.width);
        debug_assert!(self.extent.height <= texture.height);

        let scale_x = 1.0 / f32::from(texture.width);
        let scale_y = 1.0 / f32::from(texture.height);

        UvRect {
            origin: UvPoint {
                u: f32::from(self.origin.x) * scale_x,
                v: f32::from(self.origin.y) * scale_y,
            },
            extent: UvExtent {
                width: f32::from(self.extent.width) * scale_x,
                height: f32::from(self.extent.height) * scale_y,
            },
        }
    }
}

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
    text_engine: TextEngine,
}

impl Graphics {
    pub(crate) fn new(config: &GraphicsConfig) -> Self {
        let device = Device::new(config);

        let textures = TextureCache::new(
            ImageExtent::new(1, 1),
            Layout::Rgba8,
            Format::Linear,
            |extent, layout, format| device.create_texture(extent, layout, format),
        );

        let (_, white_pixel) = textures.default();

        device.copy_raster_to_texture(
            white_pixel,
            &RasterBuf::new(
                ImageInfo {
                    extent: ImageExtent::new(1, 1),
                    layout: Layout::Rgba8,
                    format: Format::Linear,
                },
                &[0xFF, 0xFF, 0xFF, 0xFF],
            ),
            TexturePoint::new(0, 0),
        );

        device.flush_upload_buffer();

        let text_engine = TextEngine::default();

        Self {
            device,
            textures,
            text_engine,
        }
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
        arena: &'a mut Arena,
        target: &'a RenderTarget,
        draw_list: &'a mut DrawList,
        scale: DpiScale,
    ) -> Canvas<'a> {
        debug_assert!(scale.factor > 0.0, "Invalid scale factor");
        let width = (target.extent().width as f32 / scale.factor).ceil();
        let height = (target.extent().height as f32 / scale.factor).ceil();

        let rect = Rect::new(Point::ORIGIN, Extent::new(width, height));

        Canvas::new(
            &self.textures,
            &self.text_engine,
            arena,
            draw_list,
            rect,
            scale,
        )
    }

    pub fn draw(&self, draw_list: &DrawList, target: &mut RenderTarget) {
        self.device.draw(draw_list, target);
    }
}
