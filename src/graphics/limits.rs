use crate::core::limits::{Limit, TexelExtentRange, Usize};

pub const IMAGE_EXTENT: TexelExtentRange<1, 1, 4096, 4096> =
    TexelExtentRange::new("Image extent out of range.");

pub const MAX_IMAGE_COUNT: Usize<4096> =
    Usize::new(|Limit(limit), value| *value < limit, "Too many images");

pub const DRAW_LIST_MAX_RUN_SIZE: Usize<{ u32::MAX as _ }> = Usize::new(
    |Limit(limit), value| *value < limit,
    "Too many items in draw list",
);
