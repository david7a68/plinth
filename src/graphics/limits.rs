use crate::core::limits::{TexelExtentRange, Usize, MAX};

pub const GFX_IMAGE_EXTENT: TexelExtentRange<1, 1, 4096, 4096> =
    TexelExtentRange::new("Image extent out of range.");

pub const GFX_IMAGE_COUNT: Usize<4096, MAX> = Usize::new("Too many images");

pub const GFX_TEXTURE_COUNT: Usize<128, MAX> = Usize::new("Too many textures");

pub const GFX_DRAW_PRIM_COUNT: Usize<{ u32::MAX as _ }, MAX> =
    Usize::new("Too many items in draw list");
