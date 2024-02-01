use std::sync::mpsc::Receiver;

use slotmap::new_key_type;

use crate::graphics::{Image, PixelBufferRef};

new_key_type! {
    pub struct LocationId;
}

impl std::fmt::Display for LocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not valid. The location may have been removed.")]
    InvalidLocation(LocationId),
    #[error("The resource was not found at the given path within the location.")]
    ResourceNotFound(LocationId),
    #[error("The resource could not be loaded due to an IO error.")]
    IoError(std::io::Error),
    #[error("The image could not be decoded.")]
    ImageDecode,
    #[error("The image exceeds the maximum size.")]
    ExceedsMaxSize,
    #[error("The resource format was not recognized or is not supported.")]
    UnknownFormat,
}

pub struct AsyncLoad<T> {
    future: Receiver<T>,
}

impl<T> AsyncLoad<T> {
    pub(crate) fn new(future: Receiver<T>) -> Self {
        Self { future }
    }

    pub fn get_async(&mut self) -> Option<T> {
        self.future.try_recv().ok()
    }

    pub fn get_blocking(&mut self) -> T {
        self.future.recv().unwrap()
    }
}

pub trait Location: Send + Sync + 'static {
    type Data: std::ops::Deref<Target = [u8]>;

    fn load(&self, path: &str) -> Result<Self::Data, Error>;
}

pub trait ImageLoader: Send + 'static {
    fn can_load(&self, bytes: &[u8]) -> bool;

    fn load<'a>(
        &'a mut self,
        bytes: &[u8],
        callback: &mut dyn FnMut(&PixelBufferRef) -> Image,
    ) -> Result<Image, Error>;
}

pub mod fs {
    use memmap2::Mmap;

    use super::{Error, Location};

    pub struct MappedFile {
        data: Mmap,
    }

    impl std::ops::Deref for MappedFile {
        type Target = [u8];

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    pub struct FileSystem {}

    impl FileSystem {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Default for FileSystem {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Location for FileSystem {
        type Data = MappedFile;

        fn load(&self, path: &str) -> Result<Self::Data, Error> {
            let file = std::fs::File::open(path).map_err(Error::IoError)?;
            let data = unsafe { memmap2::Mmap::map(&file) }.map_err(Error::IoError)?;
            Ok(MappedFile { data })
        }
    }
}

pub mod image {
    use image::{guess_format, ImageDecoder, ImageFormat};

    use crate::{
        graphics::{Image, PixelBufferRef},
        math::Size,
    };

    use super::{Error, ImageLoader};

    #[cfg(any(feature = "png", feature = "jpeg"))]
    pub struct DefaultLoader {
        buffer: Vec<u8>,
    }

    impl DefaultLoader {
        pub fn new() -> Self {
            Self {
                buffer: Vec::with_capacity(256 * 256 * 4),
            }
        }
    }

    impl Default for DefaultLoader {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ImageLoader for DefaultLoader {
        fn can_load(&self, bytes: &[u8]) -> bool {
            if let Ok(format) = guess_format(bytes) {
                match format {
                    #[cfg(feature = "png")]
                    ImageFormat::Png => true,
                    #[cfg(feature = "jpeg")]
                    ImageFormat::Jpeg => true,
                    _ => false,
                }
            } else {
                false
            }
        }

        fn load<'a>(
            &'a mut self,
            bytes: &[u8],
            callback: &mut dyn FnMut(&PixelBufferRef) -> Image,
        ) -> Result<Image, Error> {
            let format = guess_format(bytes).map_err(|_| Error::UnknownFormat)?;

            let (size, layout) = match format {
                #[cfg(feature = "png")]
                ImageFormat::Png => {
                    let decoder = image::codecs::png::PngDecoder::new(bytes)
                        .map_err(|_| Error::ImageDecode)?;

                    let (width, height) = decoder.dimensions();
                    let layout = decoder.color_type();

                    let buffer_size = decoder.total_bytes();
                    if buffer_size > isize::MAX as u64 {
                        return Err(Error::ExceedsMaxSize);
                    }

                    self.buffer.resize(buffer_size as usize, 0);

                    decoder
                        .read_image(&mut self.buffer)
                        .map_err(|_| Error::ImageDecode)?;

                    ((width, height), layout)
                }
                #[cfg(feature = "jpeg")]
                ImageFormat::Jpeg => {
                    let decoder = image::codecs::jpeg::JpegDecoder::new(bytes)
                        .map_err(|_| Error::ImageDecode)?;

                    let (width, height) = decoder.dimensions();
                    let layout = decoder.color_type();

                    let buffer_size = decoder.total_bytes();
                    if buffer_size > isize::MAX as u64 {
                        return Err(Error::ExceedsMaxSize);
                    }

                    self.buffer.resize(buffer_size as usize, 0);

                    decoder
                        .read_image(&mut self.buffer)
                        .map_err(|_| Error::ImageDecode)?;

                    ((width, height), layout)
                }
                _ => return Err(Error::UnknownFormat),
            };

            let width = u16::try_from(size.0).map_err(|_| Error::ExceedsMaxSize)?;
            let height = u16::try_from(size.1).map_err(|_| Error::ExceedsMaxSize)?;

            let layout = match layout {
                image::ColorType::Rgba8 => crate::graphics::Layout::Rgba8,
                _ => return Err(Error::UnknownFormat),
            };

            let image = callback(&PixelBufferRef {
                size: Size::new(width, height),
                layout,
                buffer: &self.buffer,
            });

            Ok(image)
        }
    }
}
