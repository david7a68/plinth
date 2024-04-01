use crate::core::limits::{Limit, TexelExtentRange, Usize};

pub const GFX_IMAGE_EXTENT: TexelExtentRange<1, 1, 4096, 4096> =
    TexelExtentRange::new("Image extent out of range.");

pub const GFX_IMAGE_COUNT: Usize<4096> =
    Usize::new(|Limit(limit), value| *value < limit, "Too many images");

pub const GFX_TEXTURE_COUNT: Usize<128> =
    Usize::new(|Limit(limit), value| *value < limit, "Too many textures");

pub const GFX_DRAW_PRIM_COUNT: Usize<{ u32::MAX as _ }> = Usize::new(
    |Limit(limit), value| *value < limit,
    "Too many items in draw list",
);
