use std::{fmt::Debug, ptr::addr_of};

use crate::{
    geometry::{Extent, Texel},
    limits::MAX_IMAGE_COUNT,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The image size exceeds the max image size.")]
    SizeLimit,
    #[error("The image could not be created because the image count limit ({}) has been reached ", MAX_IMAGE_COUNT.get())]
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
}

impl Layout {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Layout::Rgba8 => 4,
            Layout::Rgba8Vector => 4,
            Layout::Bgra8 => 4,
            Layout::Alpha8 => 1,
        }
    }
}

impl From<u8> for Layout {
    fn from(value: u8) -> Self {
        match value {
            0 => Layout::Rgba8,
            1 => Layout::Rgba8Vector,
            2 => Layout::Bgra8,
            3 => Layout::Alpha8,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Info {
    pub extent: Extent<Texel>,
    pub format: Format,
    pub layout: Layout,
    pub stride: u16,
}

impl Info {
    pub const fn row_size(&self, with_padding: bool) -> usize {
        let width = self.extent.width.0 as usize * self.layout.bytes_per_pixel();

        if with_padding {
            width.next_multiple_of(self.stride as usize)
        } else {
            width
        }
    }
}

impl Info {
    pub(crate) fn pack(&self) -> PackedInfo {
        PackedInfo::new()
            .with_width(self.extent.width.0)
            .with_height(self.extent.height.0)
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
    pub fn extent(&self) -> Extent<Texel> {
        Extent::new(Texel(self.info.width()), Texel(self.info.height()))
    }

    pub fn layout(&self) -> Layout {
        self.info.layout().into()
    }

    pub fn format(&self) -> Format {
        self.info.format().into()
    }
}

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
pub struct PixelBuf<'a> {
    info: Info,
    data: &'a [u8],
}

impl<'a> PixelBuf<'a> {
    #[must_use]
    pub const fn new(info: Info, data: &[u8]) -> PixelBuf {
        PixelBuf { info, data }
    }

    #[must_use]
    pub const fn info(&self) -> &Info {
        &self.info
    }

    #[must_use]
    pub const fn width(&self) -> Texel {
        self.info.extent.width
    }

    #[must_use]
    pub const fn height(&self) -> Texel {
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
    pub const fn row_size(&self, with_padding: bool) -> usize {
        let size = self.info.row_size(with_padding);

        #[cfg(debug_assertions)]
        {
            assert!(
                size.next_multiple_of(self.info.stride as usize) % self.info.stride as usize == 0
            );
            assert!(
                size.next_multiple_of(self.info.stride as usize)
                    * self.info.extent.height.0 as usize
                    == self.data.len()
            );
        }

        size
    }

    #[must_use]
    pub fn split_rows(&self, num_rows: impl Into<Texel>) -> (Self, Self) {
        let num_rows = num_rows.into();
        let row_size = self.row_size(true);

        debug_assert_eq!(
            row_size * self.info.extent.height.0 as usize,
            self.data.len()
        );

        let left_height = num_rows.min(self.info.extent.height);
        let left_size = row_size * left_height.0 as usize;
        let right_height = (self.info.extent.height - left_height).max(Texel(0));

        let (left, right) = self.data.split_at(left_size);

        (
            PixelBuf::new(
                Info {
                    extent: Extent::new(self.info.extent.width, left_height),
                    ..self.info
                },
                left,
            ),
            PixelBuf::new(
                Info {
                    extent: Extent::new(self.info.extent.width, right_height),
                    ..self.info
                },
                right,
            ),
        )
    }

    #[must_use]
    pub fn by_rows(&self) -> PixelRowIter<'_> {
        let ptrs = self.data.as_ptr_range();
        let pix_len = self.row_size(false);
        let advance = self.row_size(true);

        let info = Info {
            extent: Extent::new(self.info.extent.width, 1),
            format: self.info.format,
            layout: self.info.layout,
            stride: self.info.stride,
        };

        PixelRowIter {
            next_ptr: ptrs.start,
            sentinel: ptrs.end,
            advance,
            pix_len,
            info,
            _marker: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn by_rows_with_padding(&self) -> PixelRowIter<'_> {
        let ptrs = self.data.as_ptr_range();
        let pix_len = self.row_size(true);

        let info = Info {
            extent: Extent::new(self.info.extent.width, 1),
            format: self.info.format,
            layout: self.info.layout,
            stride: self.info.stride,
        };

        PixelRowIter {
            next_ptr: ptrs.start,
            sentinel: ptrs.end,
            advance: pix_len,
            pix_len,
            info,
            _marker: std::marker::PhantomData,
        }
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

pub struct PixelRowIter<'a> {
    next_ptr: *const u8,
    sentinel: *const u8,
    advance: usize,
    pix_len: usize,
    info: Info,
    _marker: std::marker::PhantomData<&'a u8>,
}

impl<'a> Iterator for PixelRowIter<'a> {
    type Item = PixelBuf<'a>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::try_from(unsafe { self.sentinel.offset_from(self.next_ptr) }).unwrap();
        let len = len / self.advance;
        (len, Some(len))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let slice = if self.next_ptr >= self.sentinel {
            None
        } else {
            let row = unsafe { std::slice::from_raw_parts(self.next_ptr, self.pix_len) };
            self.next_ptr = unsafe { self.next_ptr.add(self.advance) };
            Some(row)
        }?;

        Some(PixelBuf::new(self.info, slice))
    }
}

#[bitfield_struct::bitfield(u32)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PackedInfo {
    #[bits(12)] // max width: 4096; could take 1 bit from _empty for 8192
    pub width: i16,
    #[bits(12)] // max height: 4096; could take 1 bit from _empty for 8192
    pub height: i16,
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
    #[bits(2)]
    _empty: u8,
    #[bits(10)] // max index: 1024, could take 1 bit from _empty for 2048
    pub index: u32,
    #[bits(20)] // max epoch: 1048576
    pub epoch: u32,
}
