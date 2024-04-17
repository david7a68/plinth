use std::{fmt::Debug, ptr::addr_of};

use crate::core::limit::Limit;

use super::{limits::GFX_IMAGE_COUNT_MAX, ImageExtent};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The image size exceeds the max image size.")]
    SizeLimit,
    #[error("The image size does not agree with the number of bytes provided.")]
    SizeError,
    #[error(
        "The image could not be created because the image count limit ({}) has been reached ",
        GFX_IMAGE_COUNT_MAX
    )]
    MaxCount,
    #[error("The image handle has expired.")]
    Expired,
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
pub struct Info {
    pub extent: ImageExtent,
    pub format: Format,
    pub layout: Layout,
}

impl Info {
    #[must_use]
    pub const fn row_size(&self) -> usize {
        self.extent.width as usize * self.layout.bytes_per_pixel()
    }
}

impl Info {
    pub(crate) fn pack(self) -> PackedInfo {
        PackedInfo::new()
            .with_width(self.extent.width)
            .with_height(self.extent.height)
            .with_layout(self.layout as u8)
            .with_format(self.format as u8)
    }
}

/// A handle to an image.
///
/// Once created, images are immutable.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    pub(crate) info: PackedInfo,
    pub(crate) key: PackedKey,
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
            .field("index", &self.key.index())
            .field("epoch", &self.key.epoch())
            .finish()
    }
}

/// Non-owning reference to pixel data.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RasterBuf<'a> {
    info: Info,
    data: &'a [u8],
}

impl<'a> RasterBuf<'a> {
    #[must_use]
    pub const fn new(info: Info, data: &[u8]) -> RasterBuf {
        ImageExtent::limit_assert(info.extent);

        let row_size = info.row_size();
        assert!(
            data.len() % row_size == 0,
            "Data size does not agree with info."
        );

        RasterBuf { info, data }
    }

    pub fn try_new(info: Info, data: &'a [u8]) -> Result<RasterBuf<'a>, Error> {
        info.extent.limit_error(Error::SizeLimit)?;

        let row_size = info.row_size();
        if data.len() % row_size != 0 {
            return Err(Error::SizeError);
        }

        Ok(RasterBuf { info, data })
    }

    #[must_use]
    pub const fn info(&self) -> Info {
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
    pub const fn data(&self) -> &[u8] {
        self.data
    }

    #[must_use]
    pub const fn unwrap(self) -> (Info, &'a [u8]) {
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

#[bitfield_struct::bitfield(u32)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackedKey {
    #[bits(12)] // max index: 4095
    pub index: u32,
    #[bits(20)] // max epoch: 1048576
    pub epoch: u32,
}
