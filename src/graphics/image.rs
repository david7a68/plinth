use std::{fmt::Debug, ptr::addr_of};

use crate::geometry::{Extent, Texel};

/// The layout of pixel data in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Rgba8,
    Bgra8,
    Alpha8,
}

/// The color space that pixel data is encoded in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Unkown,
    Srgb,
    Linear,
}

/// The resizability of an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sizing {
    /// The image can be resized without loss of quality.
    Vector,
    /// The image can be resized, but may lose quality.
    Bitmap,
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
    data: u64,
}

impl Image {
    pub fn extent(&self) -> Extent<Texel> {
        todo!()
    }

    pub fn layout(&self) -> Layout {
        todo!()
    }

    pub fn format(&self) -> Format {
        todo!()
    }

    pub fn sizing(&self) -> Sizing {
        todo!()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // todo: extract details from data -dz
        f.debug_struct("Image").field("data", &self.data).finish()
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
