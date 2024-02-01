use crate::{
    inner::BuiltInDecoderImpl, DecodeOptions, Error, FrameCount, ImageDecodeBuffer, ImageInfo,
    Layout, Output,
};

pub struct Decoder<'a> {
    buffer: &'a [u8],
    options: &'a DecodeOptions,
    scratch: &'a mut ImageDecodeBuffer,
    info: ImageInfo<'a>,
}

impl<'a> BuiltInDecoderImpl<'a> for Decoder<'a> {
    const MAGIC: &'static [u8] = b"qoif";

    fn new(
        buffer: &'a [u8],
        options: &'a DecodeOptions,
        scratch: &'a mut ImageDecodeBuffer,
    ) -> Result<Self, Error> {
        assert!(buffer.starts_with(Self::MAGIC));

        let width = {
            let w = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            u16::try_from(w).map_err(|_| Error::ImageTooLarge)?
        };
        let height = {
            let h = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
            u16::try_from(h).map_err(|_| Error::ImageTooLarge)?
        };

        options.limits.check(width, height)?;

        let layout = match buffer[12] {
            3 => Layout::Rgb8,
            4 => Layout::Rgba8,
            _ => return Err(Error::UnsupportedFormat),
        };

        let color_space = match buffer[13] {
            0 => crate::ColorSpace::Srgb,
            1 => crate::ColorSpace::Linear,
            _ => return Err(Error::UnsupportedFormat),
        };

        let info = ImageInfo {
            width,
            height,
            layout,
            color_space,
            frame_count: FrameCount::One,
            icc_profile: None,
        };

        Ok(Self {
            buffer,
            options,
            scratch,
            info,
        })
    }
}

impl crate::DecoderImpl for Decoder<'_> {
    fn info(&mut self) -> &ImageInfo {
        &self.info
    }

    fn decode_frame(&mut self) -> Result<Output, Error> {
        const OP_RGB: u8 = 0b11111110;
        const OP_RGBA: u8 = 0b11111111;
        const OP_INDEX: u8 = 0b00000000;
        const OP_DIFF: u8 = 0b010000000;
        const OP_LUMA: u8 = 0b10000000;
        const OP_RUN: u8 = 0b11000000;
        const MASK2: u8 = 0b11000000;

        fn hash_pixel(pixel: [u8; 4]) -> usize {
            let r = pixel[0] as usize * 3;
            let g = pixel[1] as usize * 5;
            let b = pixel[2] as usize * 7;
            let a = pixel[3] as usize * 11;
            (r + g + b + a) % 64
        }

        let num_pixels = self.info.width as usize * self.info.height as usize;
        let mut cursor = 0;
        let mut hashmap: [[u8; 4]; 64] = [[0, 0, 0, 0]; 64];

        let mut pixels_seen = 0;
        let mut color = [0, 0, 0, 255];

        while pixels_seen <= num_pixels {
            if self.buffer[cursor] == OP_RGB {
                color[0] = self.buffer[cursor + 1];
                color[1] = self.buffer[cursor + 2];
                color[2] = self.buffer[cursor + 3];
            } else if self.buffer[cursor] == OP_RGBA {
                color[0] = self.buffer[cursor + 1];
                color[1] = self.buffer[cursor + 2];
                color[2] = self.buffer[cursor + 3];
                color[3] = self.buffer[cursor + 4];
            } else if self.buffer[cursor] & MASK2 == OP_INDEX {
            } else if self.buffer[cursor] & MASK2 == OP_DIFF {
            } else if self.buffer[cursor] & MASK2 == OP_LUMA {
            } else if self.buffer[cursor] & MASK2 == OP_RUN {
            } else {
                return Err(Error::UnsupportedFormat);
            }

            pixels_seen += 1;
        }

        todo!()
    }
}
