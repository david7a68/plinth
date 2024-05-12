mod color;
mod draw_list;
mod gl;
mod i16q3;
mod image;
pub(crate) mod limits;
mod primitives;
mod text;

use windows::Win32::Foundation::HWND;

use crate::{
    core::slotmap::new_key_type,
    geometry::{new_extent, new_point, new_rect},
    system::{PowerPreference, WindowExtent},
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use self::{
    draw_list::CommandMut,
    gl::Device,
    image::AtlasMap,
    limits::{
        GFX_ATLAS_EXTENT_MAX, GFX_ATLAS_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, GFX_IMAGE_EXTENT_MIN,
    },
};

pub(crate) use self::gl::Swapchain;
pub(crate) use self::i16q3::*;

pub use self::{
    color::Color,
    draw_list::DrawList,
    gl::RenderTarget,
    image::{Error as ImageError, Format, Image, ImageInfo, Layout, RasterBuf},
    primitives::RoundRect,
};

new_point! {
    UvPoint(u, v, f32, 0.0),
    { limit: 0.0, 1.0, "UV point out of limits" },
}

new_extent! {
    UvExtent(f32, 0.0),
    { limit: 0.0, 1.0, "UV extent out of limits" },
}

new_rect! {
    UvRect(f32, UvPoint, UvExtent),
}

impl UvRect {
    pub const ONE: Self = Self {
        origin: UvPoint { u: 0.0, v: 0.0 },
        extent: UvExtent {
            width: 1.0,
            height: 1.0,
        },
    };

    pub fn from_texture_rect(texture_rect: TextureRect, texture: TextureExtent) -> Self {
        texture_rect.uv_in(texture)
    }

    pub fn to_uvwh(&self) -> [f32; 4] {
        [
            self.origin.u,
            self.origin.v,
            self.extent.width,
            self.extent.height,
        ]
    }
}

new_key_type!(ImageId);

new_point! {
    #[derive(Eq)]
    ImagePoint(x, y, u16, 0),
    { limit: GFX_IMAGE_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, "Image point out of limits" },
}

new_extent! {
    #[derive(Eq, Hash)]
    ImageExtent(u16, 0),
    { limit: GFX_IMAGE_EXTENT_MIN, GFX_IMAGE_EXTENT_MAX, "Image extent out of limits" },
}

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

impl From<TextureExtent> for ImageExtent {
    fn from(extent: TextureExtent) -> Self {
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

new_rect! {
    ImageRect(u16, ImagePoint, ImageExtent),
}

pub use gl::TextureId;

new_point! {
    #[derive(Eq)]
    TexturePoint(x, y, u16, 0),
    { limit: GFX_ATLAS_EXTENT_MIN, GFX_ATLAS_EXTENT_MAX, "Texture point out of limits" },
}

new_extent! {
    #[derive(Eq, Hash)]
    TextureExtent(u16, 0),
    { limit: GFX_ATLAS_EXTENT_MIN, GFX_ATLAS_EXTENT_MAX, "Texture extent out of limits" },
}

new_rect! {
    TextureRect(u16, TexturePoint, TextureExtent),
}

impl From<TextureExtent> for TextureRect {
    fn from(extent: TextureExtent) -> Self {
        Self {
            origin: TexturePoint::new(0, 0),
            extent,
        }
    }
}

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

#[derive(Clone, Debug)]
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
    images: AtlasMap,
}

impl Graphics {
    pub(crate) fn new(config: &GraphicsConfig) -> Self {
        let device = Device::new(config);

        let mut images = AtlasMap::new(TextureExtent::new(1024, 1024));

        let white_pixel_info = ImageInfo {
            extent: ImageExtent::new(1, 1),
            layout: Layout::Rgba8,
            format: Format::Linear,
        };

        let white_pixel = images
            .insert(white_pixel_info, &mut |info| {
                device.create_texture(info.extent, info.layout, info.format)
            })
            .unwrap();

        assert_eq!(white_pixel.image, ImageId::from_raw(0));

        device.copy_raster_to_texture(
            white_pixel.store,
            &RasterBuf::new(white_pixel_info, &[0xFF, 0xFF, 0xFF, 0xFF]),
            TexturePoint::new(0, 0),
        );

        device.flush_upload_buffer();

        Self { device, images }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn create_swapchain(&self, hwnd: HWND) -> Swapchain {
        self.device.create_swapchain(hwnd)
    }

    pub fn create_raster_image(&mut self, info: ImageInfo) -> Result<Image, ImageError> {
        let mapping = self.images.insert(info, &mut |info| {
            self.device
                .create_texture(info.extent, info.layout, info.format)
        })?;

        let image = Image {
            info: info.pack(),
            id: mapping.image,
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
        let mapping = self.images.get(image.id)?;

        self.device
            .copy_raster_to_texture(mapping.store, pixels, mapping.place.origin);

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

    pub fn draw(&self, draw_list: &mut DrawList, target: &mut RenderTarget) {
        draw_list.close();

        for cmd in draw_list.iter_mut() {
            if let CommandMut::Rects { rects } = cmd {
                for rect in rects {
                    let mapping = self.images.get(ImageId::from_raw(rect.texture_id)).unwrap();
                    rect.uvwh = mapping.uvwh.to_uvwh();
                    rect.texture_id = mapping.store.to_raw();
                }
            }
        }

        self.device.draw(draw_list, target);
    }
}
