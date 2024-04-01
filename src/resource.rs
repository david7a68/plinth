use crate::{
    graphics::{Image, RasterBuf},
    HashedStr,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The resource path exceeds {} UTF-8 bytes.", crate::limits::RES_PATH_LENGTH.get())]
    PathTooLong,

    #[error("The path does not point to an image.")]
    NotAnImage,

    #[error("The resource could not be found.")]
    NotFound,

    #[error("The resource could not be loaded.")]
    Io(std::io::Error),
}

#[derive(Debug)]
pub enum Resource {
    Image(Image),
    // Video(Video),
}

#[derive(Debug)]
pub enum StaticResource {
    Raster(HashedStr<'static>, RasterBuf<'static>),
    // Video(HashedStr<'static>, VideoBuf<'static>), // maybe?
}
