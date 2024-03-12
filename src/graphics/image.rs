use std::{fmt::Debug, ptr::addr_of};

use crate::geometry::{Extent, Texel};

/// The layout of pixel data in memory.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Rgba8,
    Bgra8,
    Alpha8,
}

impl From<u8> for Layout {
    fn from(value: u8) -> Self {
        match value {
            0 => Layout::Rgba8,
            1 => Layout::Bgra8,
            2 => Layout::Alpha8,
            _ => panic!("Invalid layout value: {}", value),
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
            0 => Format::Unkown,
            1 => Format::Srgb,
            2 => Format::Linear,
            _ => panic!("Invalid format value: {}", value),
        }
    }
}

/// How the image responds to resizing.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sizing {
    /// The image can be resized without loss of quality.
    Vector,
    /// The image can be resized, but may lose quality.
    Bitmap,
}

impl From<u8> for Sizing {
    fn from(value: u8) -> Self {
        match value {
            0 => Sizing::Vector,
            1 => Sizing::Bitmap,
            _ => panic!("Invalid sizing value: {}", value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Info {
    pub extent: Extent<Texel>,
    pub format: Format,
    pub layout: Layout,
    pub sizing: Sizing,
    pub stride: i16,
}

/// A handle to an image.
///
/// Once created, images are immutable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    packed: PackedImage,
}

impl Image {
    pub(crate) fn new(
        extent: Extent<Texel>,
        layout: Layout,
        format: Format,
        sizing: Sizing,
        index: u32,
        epoch: u32,
    ) -> Image {
        let packed = PackedImage::new()
            .with_width(extent.width.0)
            .with_height(extent.height.0)
            .with_layout(layout as u8)
            .with_format(format as u8)
            .with_sizing(sizing as u8)
            .with_index(index)
            .with_epoch(epoch);
        Image { packed }
    }

    pub fn extent(&self) -> Extent<Texel> {
        Extent::new(Texel(self.packed.width()), Texel(self.packed.height()))
    }

    pub fn layout(&self) -> Layout {
        self.packed.layout().into()
    }

    pub fn format(&self) -> Format {
        self.packed.format().into()
    }

    pub fn sizing(&self) -> Sizing {
        self.packed.sizing().into()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("extent", &self.extent())
            .field("layout", &self.layout())
            .field("format", &self.format())
            .field("sizing", &self.sizing())
            .field("index", &self.packed.index())
            .field("epoch", &self.packed.epoch())
            .finish()
    }
}

/// Non-owning reference to pixel data.
pub struct PixelBuf<'a> {
    info: Info,
    data: &'a [u8],
}

impl<'a> PixelBuf<'a> {
    #[must_use]
    pub fn new(info: Info, data: &[u8]) -> PixelBuf {
        PixelBuf { info, data }
    }

    #[must_use]
    pub fn info(&self) -> &Info {
        &self.info
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.data
    }

    #[must_use]
    pub fn unwrap(self) -> (Info, &'a [u8]) {
        (self.info, self.data)
    }
}

impl Debug for PixelBuf<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelBuf")
            .field("info", &self.info)
            .field("data", &addr_of!(self.data))
            .finish()
    }
}

/// Packed handle to an image.
#[bitfield_struct::bitfield(u64)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackedImage {
    #[bits(12)] // max width: 4096; could take 1 bit from _empty for 8192
    width: i16,
    #[bits(12)] // max height: 4096; could take 1 bit from _empty for 8192
    height: i16,
    #[bits(3)]
    layout: u8,
    #[bits(3)]
    format: u8,
    #[bits(1)]
    sizing: u8,
    #[bits(3)]
    _empty: u8,
    #[bits(10)] // max index: 1024, could take 1 bit from _empty for 2048
    index: u32,
    #[bits(20)] // max epoch: 1048576
    epoch: u32,
}
