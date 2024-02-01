#[cfg(feature = "png")]
mod png;

#[cfg(feature = "qoi")]
mod qoi;

#[cfg(all(not(feature = "custom-decoder"), feature = "builtin-decoder"))]
use inner::DecoderImpl;

#[cfg(any(feature = "custom-decoder", not(feature = "builtin-decoder")))]
pub use inner::DecoderImpl;

#[cfg(feature = "builtin-decoder")]
use inner::BuiltInDecoderImpl;

use std::{borrow::Cow, mem::MaybeUninit, ptr::NonNull};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Rgb8,
    Rgba8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    Linear,
    Unknown,
}

pub struct Limits {
    pub max_width: u16,
    pub max_height: u16,
}

impl Limits {
    fn check(&self, width: u16, height: u16) -> Result<(), Error> {
        if width > self.max_width {
            return Err(Error::ImageTooLarge);
        }

        if height > self.max_height {
            return Err(Error::ImageTooLarge);
        }

        Ok(())
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_width: u16::MAX,
            max_height: u16::MAX,
        }
    }
}

#[derive(Default)]
pub struct DecodeOptions {
    pub limits: Limits,
    pub output_layout: Option<Layout>,
    pub output_color_space: Option<ColorSpace>,
}

pub enum Error {
    OutputBufferTooSmall,
    UnsupportedOutputCombination(Layout, ColorSpace),
    ImageTooLarge,
    UnsupportedFormat,
}

/// The number of sub-images encoded within the image container.
pub enum FrameCount {
    One,
    Many(u32),
    Unknown,
}

pub struct Output {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub frame_index: u16,
    pub bytes_written: usize,
}

pub struct ImageDecodeBuffer {
    buffer: NonNull<MaybeUninit<u8>>,
    length: isize,
    capacity: isize,
    high_water_mark: isize,
}

impl ImageDecodeBuffer {
    /// The default buffer size is 8 GiB allocated in virtual memory. Actual
    /// memory consumption will track with the maximum number of bytes used by a
    /// decode operation during its lifetime.
    pub const DEFAULT_BUFFER_SIZE: usize = 8 * (2 << 30);

    /// The default amount of virtual memory committed when the buffer is
    /// created.
    pub const DEFAULT_INIT_BUFFER_SIZE: usize = 1 << 20;

    /// The default amount of virtual memory to commit when more memory is
    /// needed;
    pub const DEFAULT_COMMIT_INCREMENT: usize = 1 << 20;

    pub fn new(buffer_size: usize, init_size: usize, commit_size: usize) -> Self {
        todo!()
    }

    pub(crate) fn reset(&mut self) {
        todo!()
    }

    pub(crate) fn get_slice(&mut self, size: usize) -> Option<&mut [MaybeUninit<u8>]> {
        todo!()
    }
}

impl Default for ImageDecodeBuffer {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_BUFFER_SIZE,
            Self::DEFAULT_INIT_BUFFER_SIZE,
            Self::DEFAULT_COMMIT_INCREMENT,
        )
    }
}

pub struct ImageInfo<'a> {
    pub width: u16,
    pub height: u16,
    pub layout: Layout,
    pub color_space: ColorSpace,
    pub frame_count: FrameCount,
    pub icc_profile: Option<Cow<'a, [u8]>>,
}

pub struct DecoderSet {
    #[cfg(feature = "custom-decoder")]
    custom: Vec<Box<dyn CustomDecoderFactory>>,
}

impl DecoderSet {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "custom-decoder")]
            custom: Vec::new(),
        }
    }

    pub fn try_decode(
        &self,
        buffer: &[u8],
        options: &DecodeOptions,
        scratch: &mut ImageDecodeBuffer,
        callback: impl FnOnce(&mut dyn DecoderImpl) -> Result<(), Error>,
    ) -> Result<(), Error> {
        #[cfg(feature = "builtin-decoder")]
        let r = self.try_decode_builtin(buffer, options, scratch, callback);

        #[cfg(all(feature = "custom-decoder", not(feature = "builtin-decoder")))]
        let r = self.try_decode_custom_only(buffer, options, scratch, callback);

        #[cfg(not(any(feature = "builtin-decoder", feature = "custom-decoder")))]
        const _: () = assert!(false, "No decoder features enabled");

        r
    }

    #[cfg(feature = "custom-decoder")]
    pub fn add_custom_decoder<T: CustomDecoderFactory>(&mut self, _factory: T) {
        self.custom.push(Box::new(_factory));
    }

    #[cfg(feature = "builtin-decoder")]
    fn try_decode_builtin(
        &self,
        buffer: &[u8],
        options: &DecodeOptions,
        scratch: &mut ImageDecodeBuffer,
        callback: impl FnOnce(&mut dyn DecoderImpl) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let Some(format) = Format::get_format(buffer) else {
            #[cfg(feature = "custom-decoder")]
            for factory in &self.custom {
                if let Some(output) = factory.try_decode(buffer, options, scratch, callback) {
                    return output;
                }
            }

            return Err(Error::UnsupportedFormat);
        };

        match format {
            #[cfg(feature = "png")]
            Format::Png => {
                let mut decoder = png::Decoder::new(buffer, options, scratch)?;
                callback(&mut decoder)
            }
            #[cfg(feature = "qoi")]
            Format::Qoi => {
                let mut decoder = qoi::Decoder::new(buffer, options, scratch)?;
                callback(&mut decoder)
            }
        }
    }

    #[cfg(all(feature = "custom-decoder", not(feature = "builtin-decoder")))]
    fn try_decode_custom_only(
        &self,
        buffer: &[u8],
        options: &DecodeOptions,
        scratch: &mut ImageDecodeBuffer,
        callback: impl FnOnce(&mut dyn DecoderImpl) -> Result<(), Error>,
    ) -> Result<(), Error> {
        todo!()
    }
}

impl Default for DecoderSet {
    fn default() -> Self {
        todo!()
    }
}

// This module is only used to allow for conditional exporting of the
// DecoderImpl trait. It is not used for anything else.
mod inner {
    use super::*;

    pub trait BuiltInDecoderImpl<'a>: DecoderImpl + Sized {
        const MAGIC: &'static [u8];

        fn new(
            buffer: &'a [u8],
            options: &'a DecodeOptions,
            scratch: &'a mut ImageDecodeBuffer,
        ) -> Result<Self, Error>;
    }

    pub trait DecoderImpl {
        fn info(&mut self) -> &ImageInfo;

        fn decode_frame(&mut self) -> Result<Output, Error>;
    }

    #[cfg(any(feature = "custom-decoder", not(feature = "builtin-decoder")))]
    impl<T> DecoderImpl for Box<T>
    where
        T: DecoderImpl,
    {
        fn info(&mut self) -> &ImageInfo {
            (**self).info()
        }

        fn decode_frame(&mut self) -> Result<Output, Error> {
            (**self).decode_frame()
        }
    }
}

#[cfg(any(feature = "custom-decoder", not(feature = "builtin-decoder")))]
pub trait CustomDecoderFactory {
    fn try_decode(
        &self,
        buffer: &[u8],
        options: &DecodeOptions,
        scratch: &mut ImageDecodeBuffer,
        callback: impl FnOnce(&mut dyn DecoderImpl) -> Result<(), Error>,
    ) -> Option<Result<Output, Error>>;
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "qoi")]
    Qoi,
}

impl Format {
    const MAGIC_BYTES: &'static [(&'static [u8], Format)] = &[
        #[cfg(feature = "png")]
        (png::Decoder::MAGIC, Format::Png),
        #[cfg(feature = "qoi")]
        (qoi::Decoder::MAGIC, Format::Qoi),
    ];

    fn get_format(buffer: &[u8]) -> Option<Self> {
        for (magic, format) in Self::MAGIC_BYTES.iter() {
            if buffer.starts_with(magic) {
                return Some(*format);
            }
        }

        None
    }
}
