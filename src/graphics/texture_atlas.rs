use arrayvec::ArrayVec;

use crate::{
    core::static_slot_map::{new_key_type, SlotMap},
    geometry::{Extent, Point, Rect, Scale, Texel, UV},
    graphics::{Format, Layout},
    limits::GFX_IMAGE_COUNT,
};

use super::backend::TextureId;

new_key_type!(CachedTextureId);

const ATLAS_EXTENT: Texel = Texel(1024);

pub struct TextureCache {
    textures: ArrayVec<AtlasMap, 16>,
    cache: Box<SlotMap<{ GFX_IMAGE_COUNT.get() }, CachedTexture, CachedTextureId>>,
}

impl TextureCache {
    pub fn new(
        extent: Extent<Texel>,
        layout: Layout,
        format: Format,
        alloc_new: impl FnMut(Extent<Texel>, Layout, Format) -> TextureId,
    ) -> Self {
        let mut this = Self {
            textures: ArrayVec::new(),
            cache: Box::new(SlotMap::new()),
        };

        let _ = this.insert_rect(extent, layout, format, alloc_new);

        this
    }

    pub fn default(&self) -> (CachedTextureId, TextureId) {
        let id = CachedTextureId::new(0, 0);
        (id, self.cache.get(id).unwrap().clone().texture)
    }

    pub fn insert_rect(
        &mut self,
        extent: Extent<Texel>,
        layout: Layout,
        format: Format,
        mut alloc_new: impl FnMut(Extent<Texel>, Layout, Format) -> TextureId,
    ) -> (TextureId, CachedTextureId) {
        // todo: actually calculate the required extent
        let texture = alloc_new(Extent::new(ATLAS_EXTENT, ATLAS_EXTENT), layout, format);

        self.textures.push(AtlasMap {
            texture,
            used: true,
        });

        let cached_id = self
            .cache
            .insert(CachedTexture {
                texture,
                rect: Rect::new(Point::new(0, 0), extent),
            })
            .unwrap();

        (texture, cached_id)
    }

    pub fn remove_rect(&mut self, image: CachedTextureId) {
        todo!()
    }

    pub fn get_rect(&self, image: CachedTextureId) -> (TextureId, Rect<Texel>) {
        let cached = self.cache.get(image).unwrap();
        (cached.texture, cached.rect)
    }

    pub fn get_uv_rect(&self, image: CachedTextureId) -> (TextureId, Rect<UV>) {
        let cached = self.cache.get(image).unwrap();
        let scale = Scale::new(1.0 / f32::from(ATLAS_EXTENT.0));
        (cached.texture, cached.rect.scale_to(scale))
    }
}

#[derive(Clone, Debug)]
pub struct CachedTexture {
    pub texture: TextureId,
    pub rect: Rect<Texel>,
}

struct AtlasMap {
    texture: TextureId,
    used: bool,
}
