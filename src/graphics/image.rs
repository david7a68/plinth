use crate::geometry::{Extent, Texel};

/// The layout of pixel data in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Rgba8,
    Bgra8,
    Alpha8,
}

/// The color space that pixel data is encoded in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Unkown,
    Srgb,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Info {
    pub format: Format,
    pub extent: Extent<Texel>,
    pub stride: i16,
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
