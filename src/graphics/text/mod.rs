#![allow(unused)]

use std::{cell::RefCell, collections::HashMap, ffi::c_void, iter::once, marker::PhantomData};

use smallvec::SmallVec;
use windows::{
    core::{implement, Error as WindowsError, IUnknown, PCWSTR},
    Win32::{
        Foundation::{GetLastError, BOOL},
        Graphics::DirectWrite::{
            DWriteCreateFactory, IDWriteFactory, IDWriteInlineObject, IDWritePixelSnapping_Impl,
            IDWriteTextFormat, IDWriteTextLayout, IDWriteTextRenderer, IDWriteTextRenderer_Impl,
            DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
            DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STYLE_OBLIQUE, DWRITE_FONT_WEIGHT_BOLD,
            DWRITE_FONT_WEIGHT_LIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_GLYPH_RUN,
            DWRITE_GLYPH_RUN_DESCRIPTION, DWRITE_MATRIX, DWRITE_MEASURING_MODE,
            DWRITE_STRIKETHROUGH, DWRITE_UNDERLINE,
        },
    },
};

use crate::{
    core::{
        arena::Arena,
        slotmap::{new_key_type, SlotMap},
        static_lru_cache::LruCache,
        PassthroughBuildHasher,
    },
    geometry::{Extent, Point, Rect},
    hashed_str,
    system::DpiScale,
    Hash, HashedStr,
};

use super::{draw_list::Primitive, gl::TextureId, Color, DrawList, RasterBuf, TextureExtent};

new_key_type!(LayoutId);

pub const CACHE_SIZE: usize = 32;

pub const DEFAULT_FONT: FontOptions<'static> = FontOptions {
    name: hashed_str!("Arial"),
    size: Pt(16),
    weight: Weight::Normal,
    shape: Shape::Normal,
    locale: hashed_str!("en-us"),
};

#[derive(Clone, Copy, Debug)]
pub enum Error {
    InvalidFormat,
    GlyphCacheFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pt(pub u8);

impl Pt {
    pub fn to_dip(self, dpi: f32) -> f32 {
        const POINTS_PER_INCH: f32 = 72.0;
        self.0 as f32 * dpi / 72.0
    }
}

pub fn layout_text(
    temp: &mut Arena,
    text: &str,
    block: TextBox,
    style: FontOptions,
    scale: DpiScale,
) -> TextLayout {
    todo!()
}

pub struct TextEngine {
    factory: IDWriteFactory,
    default_format: IDWriteTextFormat,
    cached_formats: RefCell<LruCache<CACHE_SIZE, IDWriteTextFormat>>,
}

impl TextEngine {
    pub fn new(default_font: FontOptions) -> Self {
        let factory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }.unwrap();

        let cached_formats = RefCell::new(LruCache::new());

        let default_format = create_format(&factory, default_font, DpiScale::new(1.0)).unwrap();

        Self {
            factory,
            default_format,
            cached_formats,
        }
    }

    pub fn get(&self, id: LayoutId) -> Option<&TextLayout> {
        todo!()
    }

    pub fn layout_text(
        &self,
        temp: &mut Arena,
        text: &str,
        block: TextBox,
        style: FontOptions,
        scale: DpiScale,
    ) -> TextLayout {
        let chars = {
            let mut arr = temp
                .make_array(u32::try_from(text.len()).unwrap())
                .expect("Out of temp memory");
            arr.extend(temp, text.encode_utf16());
            arr
        };

        let mut cache = self.cached_formats.borrow_mut();
        let (style, _) = cache.get_or_insert_with(Hash::of(&style), || {
            create_format(&self.factory, style, scale).unwrap()
        });

        let inner = unsafe {
            self.factory
                .CreateTextLayout(&chars, style, block.extent.width, block.extent.height)
        }
        .unwrap();

        TextLayout { inner }
    }

    pub fn tick(&self) {
        // update layout cache, evicting entries that have not been used in the
        // past N ticks.

        todo!()
    }

    pub fn rasterize<'a>(
        &self,
        arena: &'a mut Arena,
        font_id: u32,
        glyph: u16,
        size: f32,
    ) -> RasterBuf<'a> {
        todo!()
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new(DEFAULT_FONT)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weight {
    Light,
    Normal,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shape {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextWrapMode {
    None,
    Word,
    Character,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontOptions<'a> {
    pub name: HashedStr<'a>,
    pub size: Pt,
    pub weight: Weight,
    pub shape: Shape,
    pub locale: HashedStr<'a>,
}

impl Default for FontOptions<'static> {
    fn default() -> Self {
        DEFAULT_FONT
    }
}

pub struct TextLayout {
    inner: IDWriteTextLayout,
}

impl TextLayout {
    pub fn id(&self) -> LayoutId {
        todo!()
    }

    /// The number of drawn rectangles in the layout.
    ///
    /// This may be different from the number of characters in the string that
    /// created the layout due to combining characters, emoji, and other factors
    /// that affect how text is drawn.
    ///
    /// Determining the number of rects in a layout may be expensive, so the
    /// value is cached after the first time it is called.
    pub fn glyph_count(&self) -> u32 {
        todo!()
    }

    pub fn write(&self, out: &mut [Primitive]) {
        todo!()
    }
}

pub struct LayoutCache {
    layouts: SlotMap<CachedLayout, LayoutId>,
    hashmap: HashMap<u64, LayoutId, PassthroughBuildHasher>,

    epoch: u64,
    time_to_live: u64,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            layouts: SlotMap::new(),
            hashmap: HashMap::with_hasher(PassthroughBuildHasher::new()),
            epoch: 0,
            time_to_live: 4,
        }
    }

    pub fn get_or_create(
        &mut self,
        arena: &Arena,
        text: &str,
        font: FontOptions,
        area: TextBox,
        size: DpiScale,
    ) -> LayoutId {
        // let key = hash text, font, area

        /*
        if let Some(id) = self.hashmap.get(&key) {
            *id
        } else {
            let layout = layout_text(arena, text, area, font, size);
            let id = self.layouts.insert(layout);
            self.hashmap.insert(key, id);
            id
        }
        */

        todo!()
    }

    pub fn get(&self, id: LayoutId) -> Option<&TextLayout> {
        self.layouts.get(id).map(|cached| &cached.layout)
    }

    pub fn tick(&mut self) {
        self.epoch += 1;

        self.layouts.retain(|_, cached| {
            let keep = cached.epoch + self.time_to_live >= self.epoch;
            if !keep {
                // self.hashmap.remove(&cached.hash);
            }
            keep
        })
    }
}

struct CachedLayout {
    layout: TextLayout,
    epoch: u64,
    hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBox {
    pub wrap: TextWrapMode,
    pub extent: Extent,
    pub line_spacing: f32,
}

fn create_format(
    factory: &IDWriteFactory,
    font: FontOptions,
    dpi: DpiScale,
) -> Result<IDWriteTextFormat, Error> {
    let weight = match font.weight {
        Weight::Light => DWRITE_FONT_WEIGHT_LIGHT,
        Weight::Normal => DWRITE_FONT_WEIGHT_NORMAL,
        Weight::Bold => DWRITE_FONT_WEIGHT_BOLD,
    };

    let shape = match font.shape {
        Shape::Normal => DWRITE_FONT_STYLE_NORMAL,
        Shape::Italic => DWRITE_FONT_STYLE_ITALIC,
        Shape::Oblique => DWRITE_FONT_STYLE_OBLIQUE,
    };

    let size = font.size.to_dip(dpi.factor * 96.0);

    let mut font_name = font
        .name
        .encode_utf16()
        .chain(once(0))
        .collect::<SmallVec<[u16; 64]>>();

    let mut locale = font
        .locale
        .encode_utf16()
        .chain(once(0))
        .collect::<SmallVec<[u16; 64]>>();

    let text_format = unsafe {
        factory.CreateTextFormat(
            PCWSTR(font_name.as_ptr()),
            None,
            weight,
            shape,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            PCWSTR(font_name.as_ptr()),
        )
    }
    .map_err(|e| {
        println!(
            "error creating text format: HR({:?}) GLE({:?})",
            e,
            unsafe { GetLastError() }
        );

        Error::InvalidFormat
    })?;

    Ok(text_format)
}

#[implement(IDWriteTextRenderer)]
struct TextRenderer {
    dpi: DpiScale,
}

impl IDWritePixelSnapping_Impl for TextRenderer {
    fn IsPixelSnappingDisabled(
        &self,
        clientdrawingcontext: *const c_void,
    ) -> Result<BOOL, WindowsError> {
        Ok(false.into())
    }

    fn GetCurrentTransform(
        &self,
        clientdrawingcontext: *const c_void,
        transform: *mut DWRITE_MATRIX,
    ) -> Result<(), WindowsError> {
        let transform = unsafe { transform.as_mut() }.unwrap();

        transform.m11 = 1.0;
        transform.m12 = 0.0;
        transform.m21 = 0.0;
        transform.m22 = 1.0;
        transform.dx = 0.0;
        transform.dy = 0.0;

        Ok(())
    }

    fn GetPixelsPerDip(&self, clientdrawingcontext: *const c_void) -> Result<f32, WindowsError> {
        Ok(self.dpi.factor)
    }
}

impl IDWriteTextRenderer_Impl for TextRenderer {
    fn DrawGlyphRun(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        measuringmode: DWRITE_MEASURING_MODE,
        glyphrun: *const DWRITE_GLYPH_RUN,
        glyphrundescription: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        clientdrawingeffect: Option<&IUnknown>,
    ) -> Result<(), WindowsError> {
        todo!()
    }

    fn DrawUnderline(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        underline: *const DWRITE_UNDERLINE,
        clientdrawingeffect: Option<&IUnknown>,
    ) -> Result<(), WindowsError> {
        todo!()
    }

    fn DrawStrikethrough(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        strikethrough: *const DWRITE_STRIKETHROUGH,
        clientdrawingeffect: Option<&IUnknown>,
    ) -> Result<(), WindowsError> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        clientdrawingcontext: *const ::core::ffi::c_void,
        originx: f32,
        originy: f32,
        inlineobject: Option<&IDWriteInlineObject>,
        issideways: BOOL,
        isrighttoleft: BOOL,
        clientdrawingeffect: Option<&IUnknown>,
    ) -> Result<(), WindowsError> {
        Ok(())
    }
}
