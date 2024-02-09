use crate::geometry::Size;

use super::backend::{SubmitId, TextureId};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Rgba8,
}

#[derive(Clone, Copy)]
pub struct ImageHeader {
    pub size: Size<u16>,
    pub layout: Layout,
}

pub struct PixelBuffer {}

pub struct PixelBufferRef<'a> {
    pub size: Size<u16>,
    pub layout: Layout,
    pub buffer: &'a [u8],
}

pub struct Image {
    pub size: Size<u16>,
    pub layout: Layout,
    pub texture_id: TextureId,
    pub submit_id: SubmitId,
}

impl Image {
    pub(crate) fn new(
        size: Size<u16>,
        layout: Layout,
        texture_id: TextureId,
        submit_id: SubmitId,
    ) -> Self {
        Self {
            size,
            layout,
            texture_id,
            submit_id,
        }
    }

    #[must_use]
    pub fn size(&self) -> Size<u16> {
        self.size
    }

    #[must_use]
    pub fn layout(&self) -> Layout {
        self.layout
    }
}
