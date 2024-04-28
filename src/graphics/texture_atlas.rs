use arrayvec::ArrayVec;

use crate::{
    core::slotmap::SlotMap,
    graphics::{Format, Layout},
    limits::GFX_ATLAS_COUNT_MAX,
};

use super::{
    gl::TextureId, ImageExtent, ImageId, TextureExtent, TexturePoint, TextureRect, UvRect,
};

const ATLAS_TEXTURE_DIM: u16 = 1024;

pub struct TextureCache {
    textures: ArrayVec<AtlasMap, GFX_ATLAS_COUNT_MAX>,
    cache: SlotMap<CachedTexture, ImageId>,
}

impl TextureCache {
    pub fn new(
        extent: ImageExtent,
        layout: Layout,
        format: Format,
        alloc_new: impl FnMut(TextureExtent, Layout, Format) -> TextureId,
    ) -> Self {
        let mut this = Self {
            textures: ArrayVec::new(),
            cache: SlotMap::new(),
        };

        let _ = Some(this.insert_rect(extent, layout, format, alloc_new).1);

        this
    }

    pub fn default(&self) -> ImageId {
        ImageId::new(0, 0)
    }

    pub fn insert_rect(
        &mut self,
        extent: ImageExtent,
        layout: Layout,
        format: Format,
        mut alloc_new: impl FnMut(TextureExtent, Layout, Format) -> TextureId,
    ) -> (TextureId, ImageId) {
        // todo: actually calculate the required extent
        let texture = alloc_new(
            TextureExtent::new(ATLAS_TEXTURE_DIM, ATLAS_TEXTURE_DIM),
            layout,
            format,
        );

        self.textures.push(AtlasMap {
            texture,
            used: true,
        });

        let cached_id = self.cache.insert(CachedTexture {
            texture,
            rect: TextureRect::new(TexturePoint::ORIGIN, extent.into()),
        });

        (texture, cached_id)
    }

    pub fn remove_rect(&mut self, image: ImageId) {
        todo!()
    }

    pub fn get_rect(&self, image: ImageId) -> (TextureId, TextureRect) {
        let cached = self.cache.get(image).unwrap();
        (cached.texture, cached.rect)
    }

    pub fn get_uv_rect(&self, image: ImageId) -> (TextureId, UvRect) {
        let cached = self.cache.get(image).unwrap();

        (
            cached.texture,
            cached
                .rect
                .uv_in(TextureExtent::new(ATLAS_TEXTURE_DIM, ATLAS_TEXTURE_DIM)),
        )
    }
}

#[derive(Clone, Debug)]
pub struct CachedTexture {
    pub texture: TextureId,
    pub rect: TextureRect,
}

struct AtlasMap {
    texture: TextureId,
    used: bool,
}
