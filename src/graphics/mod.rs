mod canvas;
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
    text::TextEngine,
};

pub(crate) use self::gl::Swapchain;

pub use self::{
    canvas::Canvas,
    color::Color,
    draw_list::DrawList,
    gl::RenderTarget,
    image::{Error as ImageError, Format, Image, ImageInfo, Layout, RasterBuf},
    primitives::RoundRect,
    text::{
        Error as TextError, FontOptions, Pt, Shape as FontShape, TextBox, TextLayout, TextWrapMode,
        Weight as FontWeight,
    },
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
    images: AtlasMap,
    text_engine: TextEngine,
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

        let text_engine = TextEngine::default();

        Self {
            device,
            images,
            text_engine,
        }
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
        for cmd in draw_list.iter_mut() {
            match cmd {
                CommandMut::Rects { rects } => {
                    for rect in rects {
                        let mapping = self.images.get(ImageId::from_raw(rect.texture_id)).unwrap();
                        rect.uvwh = mapping.uvwh.to_uvwh();
                        rect.texture_id = mapping.store.to_raw();
                    }
                }
                CommandMut::Chars { layout, glyphs } => {
                    // todo
                }
                _ => {}
            }
        }

        self.device.draw(draw_list, target);
    }

    // pub fn draw2(&self, content: &[Canvas2]) {
    //     for canvas in content {
    //         for cmd in canvas.draw_list.iter_mut() {
    //             match cmd {
    //                 CommandMut2::Rects { rects } => {
    //                     for rect in rects {
    //                         let (texture_id, uvwh) =
    //                             self.texture_cache.get_uv_rect(rect.texture_id);
    //                         rect.uvwh = uvwh.to_uvwh();
    //                         rect.texture_id = texture_id;
    //                     }
    //                 }
    //                 CommandMut2::Chars { layout, glyphs } => {
    //                     let layout = self.text_engine.get(layout).unwrap();
    //                     layout.write(glyphs);

    //                     for glyph in glyphs {
    //                         let image_id = self.glyph_cache.get_or_insert(glyph, |glyph| {
    //                             let bitmap = self.text_engine.rasterize(arena, glyph);

    //                             let (texture, image) = self.texture_cache.insert_rect(
    //                                 bitmap.extent(),
    //                                 bitmap.layout(),
    //                                 bitmap.format(),
    //                                 |extent, layout, format| {
    //                                     self.device.create_texture(extent, layout, format)
    //                                 },
    //                             );

    //                             let (_, rect) = self.texture_cache.get_rect(image);

    //                             self.device
    //                                 .copy_raster_to_texture(texture, &bitmap, rect.origin);

    //                             image
    //                         });

    //                         let (texture_id, uvwh) = self.texture_cache.get_uv_rect(image_id);

    //                         glyph.uvwh = uvwh.to_uvwh();
    //                         glyph.texture_id = texture_id;
    //                     }
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }

    //     // todo:
    //     // 32-bit TextureId (and TextureCache)
    //     // GlyphCache (takes a glyph and returns a texture id)
    //     // allow slotmap to be heap allocated

    //     // self.device.draw2(content);
    // }
}

// struct TextureManager {
//     atlas_textures: [ArrayVec<AtlasTexture, 8>; 4],
//     // slotmap: SlotMap<ImageId, ImageLocation>,
// }

// impl TextureManager {
//     fn new(max_images: usize, max_atlas_textures: usize) -> Self {
//         todo!()
//     }

//     fn get(&self, id: ImageId) -> (TextureRect, UvRect) {
//         todo!()
//     }

//     fn insert(
//         &mut self,
//         info: ImageInfo,
//         alloc: impl FnMut(Layout, Format, Extent) -> TextureId,
//     ) -> (ImageId, TextureRect) {
//         todo!()
//     }

//     fn remove(&mut self, id: ImageId) {
//         todo!()
//     }
// }

// enum ImageLocation {
//     Atlas { index: u16, location: UvRect },
//     Owned { texture: TextureId },
// }

// struct AtlasTexture {
//     extent: TextureExtent,
//     images: ArrayVec<ImageId, 1024>,
//     texture: TextureId,
//     // other stuff
// }

// impl AtlasTexture {
//     pub fn new(texture: TextureId, extent: TextureExtent) -> Self {
//         Self {
//             extent,
//             images: ArrayVec::new(),
//             texture,
//         }
//     }
// }
