use std::{cell::Cell, fmt::Debug, ptr::addr_of};

use crate::{
    geometry::{Extent, Pixel, Point, Rect, Texel},
    limits::MAX_IMAGE_COUNT,
};

use super::Color;

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
    Alpha8Vector,
}

impl Layout {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Layout::Rgba8 => 4,
            Layout::Rgba8Vector => 4,
            Layout::Bgra8 => 4,
            Layout::Alpha8 => 1,
            Layout::Alpha8Vector => 1,
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
}

impl Info {
    pub const fn row_size(&self) -> usize {
        self.extent.width.0 as usize * self.layout.bytes_per_pixel()
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
pub struct RasterBuf<'a> {
    info: Info,
    data: &'a [u8],
}

impl<'a> RasterBuf<'a> {
    #[must_use]
    pub const fn new(info: Info, data: &[u8]) -> RasterBuf {
        RasterBuf { info, data }
    }

    #[must_use]
    pub const fn info(&self) -> Info {
        self.info
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
    pub const fn row_size(&self) -> usize {
        self.info.row_size()
    }

    #[must_use]
    pub fn split_rows(&self, num_rows: impl Into<Texel>) -> (Self, Self) {
        let num_rows = num_rows.into();
        let row_size = self.row_size();

        debug_assert_eq!(
            row_size * self.info.extent.height.0 as usize,
            self.data.len()
        );

        let left_height = num_rows.min(self.info.extent.height);
        let left_size = row_size * left_height.0 as usize;
        let right_height = (self.info.extent.height - left_height).max(Texel(0));

        let (left, right) = self.data.split_at(left_size);

        (
            RasterBuf::new(
                Info {
                    extent: Extent::new(self.info.extent.width, left_height),
                    ..self.info
                },
                left,
            ),
            RasterBuf::new(
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
        let row_len = self.row_size();

        let info = Info {
            extent: Extent::new(self.info.extent.width, 1),
            format: self.info.format,
            layout: self.info.layout,
        };

        PixelRowIter {
            next_ptr: ptrs.start,
            sentinel: ptrs.end,
            row_len,
            info,
            _marker: std::marker::PhantomData,
        }
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

pub struct PixelRowIter<'a> {
    next_ptr: *const u8,
    sentinel: *const u8,
    row_len: usize,
    info: Info,
    _marker: std::marker::PhantomData<&'a u8>,
}

impl<'a> Iterator for PixelRowIter<'a> {
    type Item = RasterBuf<'a>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = usize::try_from(unsafe { self.sentinel.offset_from(self.next_ptr) }).unwrap();
        let len = len / self.row_len;
        (len, Some(len))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let slice = if self.next_ptr >= self.sentinel {
            None
        } else {
            let row = unsafe { std::slice::from_raw_parts(self.next_ptr, self.row_len) };
            self.next_ptr = unsafe { self.next_ptr.add(self.row_len) };
            Some(row)
        }?;

        Some(RasterBuf::new(self.info, slice))
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum Winding {
    NonZero,
    EvenOdd,
}

#[repr(u8)]
#[derive(Debug)]
pub enum Join {
    Bevel,
    Miter,
    Round,
}

#[repr(u8)]
#[derive(Debug)]
pub enum Cap {
    Butt,
    Square,
    Round,
}

#[derive(Debug)]
pub struct FillStyle {
    pub color: Color,
    pub winding: Winding,
}

#[derive(Debug)]
pub struct StrokeStyle {
    pub width: f32,
    pub color: Color,
    pub dash_length: f32,
    pub join: Join,
    pub cap: Cap,
    pub scale_width: bool,
}

#[derive(Debug)]
pub enum Style {
    Fill(FillStyle),
    Stroke(StrokeStyle),
}

#[derive(Debug)]
pub struct Path {
    pub start: u32,
    pub count: u32,
    pub point: u32,
    pub style: Style,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verb {
    MoveTo,
    LineTo,
    CurveTo,
    QuadTo,
    Close,
}

pub enum Command {
    MoveTo(Point<Pixel>),
    LineTo(Point<Pixel>),
    CurveTo(Point<Pixel>, Point<Pixel>, Point<Pixel>),
    QuadTo(Point<Pixel>, Point<Pixel>),
    Close,
    Fill(FillStyle),
    Stroke(StrokeStyle),
}

#[derive(Debug)]
pub struct VectorBuf<'a> {
    paths: &'a [Path],
    verbs: &'a [Verb],
    points: &'a [Point<Pixel>],

    layout: Layout,
    format: Format,

    base: Cell<Point<Pixel>>,
    size: Cell<Extent<Texel>>,
}

impl<'a> VectorBuf<'a> {
    const fn new(
        paths: &'a [Path],
        verbs: &'a [Verb],
        points: &'a [Point<Pixel>],
        layout: Layout,
        format: Format,
    ) -> VectorBuf<'a> {
        VectorBuf {
            verbs,
            points,
            paths,
            layout,
            format,
            size: Cell::new(Extent::ZERO),
            base: Cell::new(Point::ZERO),
        }
    }

    pub fn info(&self) -> Info {
        if self.size.get() == Extent::ZERO {
            let (base, size) = self
                .points
                .iter()
                .fold((Point::ZERO, Point::ZERO), |(min, max), point| {
                    (min.min(point), max.max(point))
                });

            self.size.set(
                Extent {
                    width: Texel(size.x.0.ceil() as i16),
                    height: Texel(size.y.0.ceil() as i16),
                }
                .max(&Extent::ONE),
            );

            self.base.set(base);
        }

        Info {
            extent: self.size.get(),
            layout: self.layout,
            format: self.format,
        }
    }

    pub fn bounds(&self) -> Rect<Pixel> {
        let extent = self.info().extent.cast();
        Rect::new(self.base.get(), extent)
    }

    pub fn paths(&self) -> &[Path] {
        self.paths
    }

    pub fn iter_paths(&self) -> PathIter<'a> {
        PathIter {
            paths: self.paths.iter(),
            verbs: self.verbs,
            points: self.points,
        }
    }
}

pub struct PathRef<'a> {
    style: &'a Style,
    verbs: &'a [Verb],
    points: &'a [Point<Pixel>],
}

impl PathRef<'_> {
    pub fn style(&self) -> &Style {
        self.style
    }

    pub fn commands(&self) -> CommandIter<'_> {
        CommandIter {
            verbs: self.verbs.iter(),
            points: self.points,
            n_points: 0,
        }
    }
}

pub struct PathIter<'a> {
    paths: std::slice::Iter<'a, Path>,
    verbs: &'a [Verb],
    points: &'a [Point<Pixel>],
}

impl<'a> Iterator for PathIter<'a> {
    type Item = PathRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let path = self.paths.next()?;
        let verbs = &self.verbs[path.start as usize..];
        let points = &self.points[path.point as usize..];

        Some(PathRef {
            verbs,
            points,
            style: &path.style,
        })
    }
}

pub struct CommandIter<'a> {
    verbs: std::slice::Iter<'a, Verb>,
    points: &'a [Point<Pixel>],
    n_points: usize,
}

impl Iterator for CommandIter<'_> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let (command, points) = match self.verbs.next()? {
            Verb::MoveTo => (Command::MoveTo(self.points[self.n_points]), 1),
            Verb::LineTo => (Command::LineTo(self.points[self.n_points]), 1),
            Verb::CurveTo => (
                Command::CurveTo(
                    self.points[self.n_points],
                    self.points[self.n_points + 1],
                    self.points[self.n_points + 2],
                ),
                3,
            ),
            Verb::QuadTo => (
                Command::QuadTo(self.points[self.n_points], self.points[self.n_points + 1]),
                2,
            ),
            Verb::Close => (Command::Close, 0),
        };

        self.n_points += points;
        Some(command)
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
    #[bits(12)] // max index: 4095
    pub index: u32,
    #[bits(20)] // max epoch: 1048576
    pub epoch: u32,
}
