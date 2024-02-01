use crate::{inner::BuiltInDecoderImpl, DecodeOptions, Error, ImageDecodeBuffer};

pub struct Decoder<'a> {
    buffer: &'a [u8],
    options: &'a DecodeOptions,
    scratch: &'a mut ImageDecodeBuffer,
}

impl<'a> BuiltInDecoderImpl<'a> for Decoder<'a> {
    const MAGIC: &'static [u8] = b"\x89PNG\r\n\x1a\n";

    fn new(
        buffer: &'a [u8],
        options: &'a DecodeOptions,
        scratch: &'a mut ImageDecodeBuffer,
    ) -> Result<Self, Error> {
        Ok(Self {
            buffer,
            options,
            scratch,
        })
    }
}

impl crate::DecoderImpl for Decoder<'_> {
    fn info(&mut self) -> &crate::ImageInfo {
        todo!()
    }

    fn decode_frame(&mut self) -> Result<crate::Output, crate::Error> {
        todo!()
    }
}
