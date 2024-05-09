use std::{fmt::Debug, ptr::addr_of};

use arrayvec::ArrayVec;

use crate::core::{limit::Limit, slotmap::SlotMap};

use super::{
    limits::{GFX_ATLAS_COUNT_MAX, GFX_IMAGE_COUNT_MAX},
    ImageExtent, ImageId, TextureExtent, TextureId, TexturePoint, TextureRect, UvRect,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The image size exceeds the max image size.")]
    SizeLimit,
    #[error("The image size does not agree with the number of bytes provided.")]
    SizeError,
    #[error(
        "The image could not be created because the image count limit ({}) has been reached.",
        GFX_IMAGE_COUNT_MAX
    )]
    MaxCount,
    #[error("The image handle has expired.")]
    Expired,
    #[error("The combined image layout and format cannot be used to store an image.")]
    IncompatibleStorage(Layout, Format),
}

/// The layout of pixel data in memory.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Rgba8,
    Rgba8Vector,
    Bgra8,
    Alpha8,
    Alpha8Vector,
}

impl Layout {
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8 | Self::Rgba8Vector | Self::Bgra8 => 4,
            Self::Alpha8 | Self::Alpha8Vector => 1,
        }
    }
}

impl From<u8> for Layout {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Rgba8,
            1 => Self::Rgba8Vector,
            2 => Self::Bgra8,
            3 => Self::Alpha8,
            _ => panic!("Invalid layout value: {value}"),
        }
    }
}

/// The color space that pixel data is encoded in.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Unkown,
    Srgb,
    Linear,
}

impl From<u8> for Format {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unkown,
            1 => Self::Srgb,
            2 => Self::Linear,
            _ => panic!("Invalid format value: {value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageInfo {
    pub extent: ImageExtent,
    pub format: Format,
    pub layout: Layout,
}

impl ImageInfo {
    #[must_use]
    pub const fn row_size(&self) -> usize {
        self.extent.width as usize * self.layout.bytes_per_pixel()
    }
}

impl ImageInfo {
    pub(crate) fn pack(self) -> PackedInfo {
        PackedInfo::new()
            .with_width(self.extent.width)
            .with_height(self.extent.height)
            .with_layout(self.layout as u8)
            .with_format(self.format as u8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureInfo {
    pub extent: TextureExtent,
    pub format: Format,
    pub layout: Layout,
}

/// A handle to an image.
///
/// Once created, images are immutable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    pub(crate) info: PackedInfo,
    pub(crate) id: ImageId,
}

impl Image {
    #[must_use]
    pub fn extent(&self) -> ImageExtent {
        ImageExtent::new(self.info.width(), self.info.height())
    }

    #[must_use]
    pub fn layout(&self) -> Layout {
        self.info.layout().into()
    }

    #[must_use]
    pub fn format(&self) -> Format {
        self.info.format().into()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("extent", &self.extent())
            .field("layout", &self.layout())
            .field("format", &self.format())
            .field("index", &self.id.index())
            .field("epoch", &self.id.epoch())
            .finish()
    }
}

/// Non-owning reference to pixel data.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RasterBuf<'a> {
    info: ImageInfo,
    data: &'a [u8],
}

impl<'a> RasterBuf<'a> {
    #[must_use]
    pub const fn new(info: ImageInfo, data: &[u8]) -> RasterBuf {
        ImageExtent::limit_assert(info.extent);

        let row_size = info.row_size();
        assert!(
            data.len() % row_size == 0,
            "Data size does not agree with info."
        );

        RasterBuf { info, data }
    }

    pub fn try_new(info: ImageInfo, data: &'a [u8]) -> Result<RasterBuf<'a>, Error> {
        info.extent.limit_error(Error::SizeLimit)?;

        let row_size = info.row_size();
        if data.len() % row_size != 0 {
            return Err(Error::SizeError);
        }

        Ok(RasterBuf { info, data })
    }

    #[must_use]
    pub const fn info(&self) -> ImageInfo {
        self.info
    }

    #[must_use]
    pub const fn width(&self) -> u16 {
        self.info.extent.width
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.info.extent.height
    }

    #[must_use]
    pub const fn extent(&self) -> ImageExtent {
        self.info.extent
    }

    #[must_use]
    pub const fn format(&self) -> Format {
        self.info.format
    }

    #[must_use]
    pub const fn layout(&self) -> Layout {
        self.info.layout
    }

    #[must_use]
    pub const fn data(&self) -> &[u8] {
        self.data
    }

    #[must_use]
    pub const fn unwrap(self) -> (ImageInfo, &'a [u8]) {
        (self.info, self.data)
    }

    #[must_use]
    pub const fn row_size(&self) -> usize {
        self.info.row_size()
    }
}

impl Debug for RasterBuf<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelBuf")
            .field("info", &self.info)
            .field("data", &addr_of!(self.data))
            .finish()
    }
}

#[bitfield_struct::bitfield(u32)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackedInfo {
    #[bits(12)] // max width: 4096; could take 1 bit from _empty for 8192
    pub width: u16,
    #[bits(12)] // max height: 4096; could take 1 bit from _empty for 8192
    pub height: u16,
    #[bits(3)]
    pub layout: u8,
    #[bits(3)]
    pub format: u8,
    #[bits(2)]
    _empty: u8,
}

pub struct ImageMapping {
    pub image: ImageId,
    pub store: TextureId,
    pub place: TextureRect,
    pub uvwh: UvRect,
}

pub struct AtlasMap {
    map: SlotMap<Storage, ImageId>,
    extent: TextureExtent,
    textures: [ArrayVec<Atlas, GFX_ATLAS_COUNT_MAX>; 3],
}

impl AtlasMap {
    pub fn new(atlas_texture_extent: TextureExtent) -> Self {
        Self {
            extent: atlas_texture_extent,
            textures: [
                // rgba8 linear
                ArrayVec::new(),
                // rgba8 srgb
                ArrayVec::new(),
                // alpha8 linear
                ArrayVec::new(),
            ],
            map: SlotMap::with_capacity(GFX_IMAGE_COUNT_MAX),
        }
    }

    pub fn get(&self, image: ImageId) -> Result<ImageMapping, Error> {
        let (store, place, uvwh) = match self.map.get(image).ok_or(Error::Expired)? {
            Storage::Atlas {
                place,
                index,
                storage,
            } => {
                let textures = &self.textures[usize::from(*storage)];
                let atlas = textures.get(usize::from(*index)).unwrap();
                let uvwh = UvRect::from_texture_rect(*place, atlas.extent);
                (atlas.texture, *place, uvwh)
            }
            Storage::Owned { texture, extent } => (
                *texture,
                TextureRect::new(TexturePoint::ORIGIN, *extent),
                UvRect::ONE,
            ),
        };

        Ok(ImageMapping {
            image,
            store,
            place,
            uvwh,
        })
    }

    pub fn insert(
        &mut self,
        image: ImageInfo,
        alloc: &mut dyn FnMut(TextureInfo) -> TextureId,
    ) -> Result<ImageMapping, Error> {
        image.extent.limit_error(Error::SizeLimit)?;

        if image.extent.width > self.extent.width || image.extent.height > self.extent.height {
            let store = alloc(TextureInfo {
                extent: self.extent,
                format: image.format,
                layout: image.layout,
            });

            let image = self.map.insert(Storage::Owned {
                texture: store,
                extent: TextureExtent::new(image.extent.width, image.extent.height),
            });

            return Ok(ImageMapping {
                image,
                store,
                place: TextureRect::new(TexturePoint::ORIGIN, self.extent),
                uvwh: UvRect::ONE,
            });
        }

        let storage_id = Self::to_storage_index(image.layout, image.format);
        let textures = &mut self.textures[usize::from(storage_id)];

        for (i, texture) in textures.iter_mut().enumerate() {
            if let Ok(place) = texture.add_image(image.extent) {
                let index = u8::try_from(i).unwrap();
                let image = self.map.insert(Storage::Atlas {
                    place,
                    index,
                    storage: storage_id,
                });

                return Ok(ImageMapping {
                    image,
                    store: texture.texture,
                    place,
                    uvwh: UvRect::from_texture_rect(place, texture.extent),
                });
            }
        }

        let texture = alloc(TextureInfo {
            extent: self.extent,
            format: image.format,
            layout: image.layout,
        });

        let mut atlas = Atlas::new(texture, self.extent);

        let place = atlas.add_image(image.extent).unwrap();

        let index = textures.len();
        textures.push(atlas);

        let image = self.map.insert(Storage::Atlas {
            place,
            index: u8::try_from(index).unwrap(),
            storage: storage_id,
        });

        Ok(ImageMapping {
            image,
            store: texture,
            place,
            uvwh: UvRect::from_texture_rect(place, self.extent),
        })
    }

    pub fn remove(&mut self, image: ImageId, free: &mut dyn FnMut(TextureId)) -> Result<(), Error> {
        #[allow(unused_variables)]
        match self.map.remove(image) {
            Some(Storage::Atlas {
                place: rect,
                index,
                storage,
            }) => {
                let index = usize::from(index);
                let textures = self.textures.get_mut(usize::from(storage)).unwrap();
                let atlas = textures.get_mut(index).unwrap();

                atlas.remove(rect);
                if atlas.is_empty() {
                    // memory saving opportunity!
                    // free(atlas.texture);
                }
            }
            Some(Storage::Owned { texture, extent: _ }) => {
                free(texture);
            }
            None => return Err(Error::Expired),
        }

        Ok(())
    }

    pub fn repack(&mut self) {
        todo!()
    }

    fn to_storage_index(layout: Layout, format: Format) -> u8 {
        match (layout, format) {
            (Layout::Rgba8, Format::Linear) => 0,
            (Layout::Rgba8, Format::Srgb) => 1,
            (Layout::Alpha8, Format::Linear) => 2,
            _ => panic!("Invalid layout and format combination."),
        }
    }
}

enum Storage {
    Atlas {
        place: TextureRect,
        index: u8,
        storage: u8,
    },
    Owned {
        texture: TextureId,
        extent: TextureExtent,
    },
}

struct Atlas {
    texture: TextureId,

    extent: TextureExtent,

    /// The x position to place the next image.
    start_x: u16,
    /// The y position to place the next image.
    start_y: u16,

    /// The y position to start the next row.
    max_height: u16,
}

impl Atlas {
    fn new(texture: TextureId, extent: TextureExtent) -> Self {
        Self {
            texture,
            extent,
            start_x: 0,
            start_y: 0,
            max_height: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.start_x == 0 && self.start_y == 0
    }

    fn add_image(&mut self, image: ImageExtent) -> Result<TextureRect, Error> {
        if image.width > self.extent.width || image.height > self.extent.height {
            return Err(Error::SizeLimit);
        }

        let (start_x, start_y) = if self.extent.width - self.start_x < image.width {
            (0, self.max_height)
        } else {
            (self.start_x, self.start_y)
        };

        if start_y + image.height > self.extent.height {
            return Err(Error::SizeLimit);
        }

        self.start_x = start_x + image.width;
        self.start_y = start_y;
        self.max_height = self.max_height.max(start_y + image.height);

        Ok(TextureRect::new(
            TexturePoint::new(start_x, start_y),
            TextureExtent::new(image.width, image.height),
        ))
    }

    fn remove(&mut self, rect: TextureRect) {
        let _ = rect;
        todo!()
    }
}
