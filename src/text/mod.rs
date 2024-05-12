use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    core::{
        arena::{Arena, Array},
        PassthroughBuildHasher, PassthroughHashMap,
    },
    geometry::{Extent, Point},
    graphics::{Color, DrawList, Graphics, ImageId, I16Q3},
    system::Dpi,
};

#[cfg(target_os = "windows")]
type Char = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pt(pub I16Q3);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Thin = 0,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextWrapMode {
    None,
    Word,
    Char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipMode {
    None,
    Ellipsis,
    Clip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

pub enum LocalNumberSubstitution {
    None,
    National,
    Contextual,
    Traditional,
}

pub struct Text<'a> {
    pub locale: &'static str,
    pub direction: TextDirection,
    pub codepoints: Array<'a, Char>,
}

impl<'a> Text<'a> {
    pub fn from_str(
        arena: &'a Arena,
        text: &str,
        locale: &'static str,
        direction: TextDirection,
    ) -> Self {
        // let codepoints = arena.make_array_from(text.encode_utf16()).unwrap();
        let mut codepoints = Array::new_in(arena);
        codepoints.extend(text.encode_utf16());

        Self {
            locale,
            direction,
            codepoints,
        }
    }
}

impl TextView for Text<'_> {
    fn length(&self) -> usize {
        self.codepoints.len() as usize
    }

    fn locale(&self, starting_from: usize) -> (&str, &[Char]) {
        (self.locale, &self.codepoints[starting_from..])
    }

    fn direction(&self) -> TextDirection {
        self.direction
    }

    fn chars_after(&self, index: usize) -> Option<&[Char]> {
        if index < self.codepoints.len() as usize {
            Some(&self.codepoints[index..])
        } else {
            None
        }
    }

    fn chars_before(&self, index: usize) -> Option<&[Char]> {
        if index < self.codepoints.len() as usize {
            Some(&self.codepoints[..index])
        } else {
            None
        }
    }
}

pub trait TextView {
    fn length(&self) -> usize;

    fn locale(&self, starting_from: usize) -> (&str, &[Char]);

    fn direction(&self) -> TextDirection;

    fn chars_after(&self, index: usize) -> Option<&[Char]>;

    fn chars_before(&self, index: usize) -> Option<&[Char]>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextStyle<'a> {
    font_weight: FontWeight,
    font_style: FontStyle,
    font_name: &'a str,
    font_size: Pt,
    text_wrap: TextWrapMode,
    clip_mode: ClipMode,
}

pub struct TextLayout {
    layout: platform::Layout,
}

impl TextLayout {}

pub struct TextEngine {
    engine: platform::Engine,
}

impl TextEngine {
    pub fn new(locale: &str) -> Self {
        let engine = platform::Engine::new();

        Self { engine }
    }

    pub fn compute_layout(
        &self,
        text: &dyn TextView,
        area: Extent,
        size: Dpi,
        style: &TextStyle,
    ) -> TextLayout {
        let layout = self.engine.compute_layout(text, area, size, style);
        TextLayout { layout }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_layout(
        &self,
        temp: &Arena,
        graphics: &Graphics,
        layout: &TextLayout,
        origin: Point,
        color: Color,
        glyphs: &mut GlyphCache,
        target: &mut DrawList,
    ) {
        self.engine.draw_layout(
            temp,
            graphics,
            &layout.layout,
            origin,
            color,
            glyphs,
            target,
        );
    }
}

pub struct GlyphCache {
    map: PassthroughHashMap<ImageId>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_hasher(PassthroughBuildHasher::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, PassthroughBuildHasher::new()),
        }
    }

    pub fn get_glyph(&mut self, style: &TextStyle, dpi: Dpi, glyph: u16) -> Option<ImageId> {
        let hash = {
            let mut hasher = DefaultHasher::new();
            style.hash(&mut hasher);
            glyph.hash(&mut hasher);
            dpi.hash(&mut hasher);
            hasher.finish()
        };

        self.map.get(&hash).cloned()
    }

    pub fn add_glyph(&mut self, style: &TextStyle, dpi: Dpi, glyph: u16, image: ImageId) {
        let hash = {
            let mut hasher = DefaultHasher::new();
            style.hash(&mut hasher);
            glyph.hash(&mut hasher);
            dpi.hash(&mut hasher);
            hasher.finish()
        };

        self.map.insert(hash, image);
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::{fmt::Debug, mem::ManuallyDrop};

    use arrayvec::ArrayVec;
    use bitfield_struct::bitfield;
    use windows::Win32::{
        Foundation::E_INVALIDARG,
        Graphics::DirectWrite::{
            DWriteCreateFactory, IDWriteFactory2, IDWriteFont, IDWriteFontCollection,
            IDWriteFontFallback, IDWriteNumberSubstitution, IDWriteTextAnalysisSink,
            IDWriteTextAnalysisSink_Impl, IDWriteTextAnalysisSource,
            IDWriteTextAnalysisSource_Impl, IDWriteTextAnalyzer, IDWriteTextLayout,
            DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_UNDEFINED,
            DWRITE_FONT_STYLE, DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STYLE_OBLIQUE, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_BLACK,
            DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_EXTRA_BOLD, DWRITE_FONT_WEIGHT_EXTRA_LIGHT,
            DWRITE_FONT_WEIGHT_LIGHT, DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_FONT_WEIGHT_THIN, DWRITE_LINE_BREAKPOINT,
            DWRITE_NUMBER_SUBSTITUTION_METHOD_FROM_CULTURE, DWRITE_READING_DIRECTION,
            DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
            DWRITE_SCRIPT_ANALYSIS,
        },
    };
    use windows_core::{implement, w, PCWSTR};

    use super::*;

    fn translate_font_weights(weight: FontWeight) -> DWRITE_FONT_WEIGHT {
        const TABLE: [DWRITE_FONT_WEIGHT; 9] = [
            DWRITE_FONT_WEIGHT_THIN,
            DWRITE_FONT_WEIGHT_EXTRA_LIGHT,
            DWRITE_FONT_WEIGHT_LIGHT,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_WEIGHT_MEDIUM,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            DWRITE_FONT_WEIGHT_BOLD,
            DWRITE_FONT_WEIGHT_EXTRA_BOLD,
            DWRITE_FONT_WEIGHT_BLACK,
        ];

        TABLE[weight as usize]
    }

    fn translate_font_styles(style: FontStyle) -> DWRITE_FONT_STYLE {
        match style {
            FontStyle::Normal => DWRITE_FONT_STYLE_NORMAL,
            FontStyle::Italic => DWRITE_FONT_STYLE_ITALIC,
            FontStyle::Oblique => DWRITE_FONT_STYLE_OBLIQUE,
        }
    }

    pub struct Layout {
        layout: IDWriteTextLayout,
    }

    pub struct Engine {
        factory: IDWriteFactory2,
        fontset: IDWriteFontCollection,
        fallback: IDWriteFontFallback,
        analyzer: IDWriteTextAnalyzer,
        substitution: IDWriteNumberSubstitution,
    }

    impl Engine {
        pub fn new() -> Self {
            let factory: IDWriteFactory2 =
                unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }.unwrap();

            let fontset = {
                let mut fontset = None;
                unsafe { factory.GetSystemFontCollection(&mut fontset, true) }.unwrap();
                fontset.unwrap()
            };

            let fallback = unsafe { factory.GetSystemFontFallback() }.unwrap();

            let analyzer = unsafe { factory.CreateTextAnalyzer() }.unwrap();

            let substitution = unsafe {
                factory.CreateNumberSubstitution(
                    DWRITE_NUMBER_SUBSTITUTION_METHOD_FROM_CULTURE,
                    w!("en-us"),
                    false,
                )
            }
            .unwrap();

            Self {
                factory,
                fontset,
                fallback,
                analyzer,
                substitution,
            }
        }

        pub fn compute_layout(
            &self,
            text: &dyn TextView,
            area: Extent,
            size: Dpi,
            style: &TextStyle,
        ) -> Layout {
            let view = WCharView {
                view: text,
                number_substitution: &self.substitution,
            };

            let font = {
                let mut buf: ArrayVec<u16, 256> = ArrayVec::new();
                buf.extend(style.font_name.encode_utf16());
                buf.push(0);
                buf
            };

            let weight = translate_font_weights(style.font_weight);
            let style = translate_font_styles(style.font_style);

            let view = IDWriteTextAnalysisSource::from(view);

            let mut index = 0;

            let mut mapped_length = 0;
            let mut mapped_scale = 0.0;
            let mut mapped_font = None;

            while mapped_length < index {
                unsafe {
                    self.fallback.MapCharacters(
                        &view,
                        index,
                        text.length() as u32,
                        &self.fontset,
                        PCWSTR::from_raw(font.as_ptr()),
                        weight,
                        style,
                        DWRITE_FONT_STRETCH_UNDEFINED,
                        &mut mapped_length,
                        &mut mapped_font,
                        &mut mapped_scale,
                    )
                }
                .unwrap();

                let font = mapped_font.as_ref().unwrap();

                let mut metrics = DWRITE_FONT_METRICS::default();
                unsafe { font.GetMetrics(&mut metrics) };

                // estimate the number of glyphs needed for a line

                // write all the glyphs to memory,
                // retrieve advances and offsets
                // iterate over the glyphs and perform line breaks and wrapping as needed
                // store glyph runs in a list

                index += mapped_length;
            }

            // fallback::map_characters

            // analyzer::analyze_script

            // analyzer::get_glyphs

            // analyzer::get_glyph_placements

            todo!()
        }

        #[allow(clippy::too_many_arguments)]
        pub fn draw_layout(
            &self,
            temp: &Arena,
            graphics: &Graphics,
            layout: &Layout,
            origin: Point,
            color: Color,
            glyphs: &mut GlyphCache,
            target: &mut DrawList,
        ) {
            // glyph run

            // factory::translate_color_glyph_run

            // factory::create_glyph_run_analysis

            // for glyph in run:
            //     if in glyph cache
            //         use it
            //     else
            //         rasterize and add to cache
            //     draw glyph

            todo!()
        }
    }

    #[implement(IDWriteTextAnalysisSource)]
    struct WCharView<'a> {
        view: &'a dyn TextView,
        number_substitution: &'a IDWriteNumberSubstitution,
    }

    impl<'a> IDWriteTextAnalysisSource_Impl for WCharView<'a> {
        fn GetTextAtPosition(
            &self,
            textposition: u32,
            textstring: *mut *mut u16,
            textlength: *mut u32,
        ) -> windows_core::Result<()> {
            let chars = self
                .view
                .chars_after(textposition as usize)
                .ok_or(E_INVALIDARG)?;

            unsafe {
                *textstring = chars.as_ptr() as *mut u16;
                *textlength = chars.len() as u32;
            }

            Ok(())
        }

        fn GetTextBeforePosition(
            &self,
            textposition: u32,
            textstring: *mut *mut u16,
            textlength: *mut u32,
        ) -> windows_core::Result<()> {
            let chars = self
                .view
                .chars_before(textposition as usize)
                .ok_or(E_INVALIDARG)?;

            unsafe {
                *textstring = chars.as_ptr() as *mut u16;
                *textlength = chars.len() as u32;
            }

            Ok(())
        }

        fn GetParagraphReadingDirection(&self) -> DWRITE_READING_DIRECTION {
            match self.view.direction() {
                TextDirection::LeftToRight => DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
                TextDirection::RightToLeft => DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
            }
        }

        fn GetLocaleName(
            &self,
            _textposition: u32,
            textlength: *mut u32,
            localename: *mut *mut u16,
        ) -> windows_core::Result<()> {
            unsafe { textlength.write(6) };
            unsafe { localename.write(w!("en-us").as_ptr().cast_mut()) };
            Ok(())
        }

        fn GetNumberSubstitution(
            &self,
            textposition: u32,
            textlength: *mut u32,
            numbersubstitution: *mut Option<IDWriteNumberSubstitution>,
        ) -> windows_core::Result<()> {
            unsafe {
                textlength.write(self.view.chars_after(textposition as usize).unwrap().len() as u32)
            };
            unsafe { numbersubstitution.write(Some(self.number_substitution.clone())) };

            Ok(())
        }
    }

    union AnalysisData {
        backup: ManuallyDrop<IDWriteFont>,
        script: DWRITE_SCRIPT_ANALYSIS,
        number: ManuallyDrop<IDWriteNumberSubstitution>,
        is_rtl: bool,
    }

    #[derive(Debug)]
    enum AnalysisKind {
        FontBackup = 0,
        IsRightToLeft = 1,
        ScriptAnalysis = 2,
        NumberSubstitution = 3,
    }

    impl AnalysisKind {
        const fn into_bits(self) -> u32 {
            self as u32
        }

        const fn from_bits(value: u32) -> Self {
            match value {
                0 => Self::FontBackup,
                1 => Self::IsRightToLeft,
                2 => Self::ScriptAnalysis,
                3 => Self::NumberSubstitution,
                _ => unreachable!(),
            }
        }
    }

    #[bitfield(u32)]
    struct Meta {
        #[bits(2)]
        kind: AnalysisKind,
        #[bits(30)]
        text: u32,
    }

    struct AnalysisResult {
        data: AnalysisData,
        more: f32,
        meta: Meta,
    }

    impl AnalysisResult {
        fn backup_font(position: u32, font: IDWriteFont, scale: f32) -> Self {
            Self {
                data: AnalysisData {
                    backup: ManuallyDrop::new(font),
                },
                more: scale,
                meta: Meta::new()
                    .with_kind(AnalysisKind::FontBackup)
                    .with_text(position),
            }
        }

        fn script_analysis(position: u32, script: DWRITE_SCRIPT_ANALYSIS) -> Self {
            Self {
                data: AnalysisData { script },
                more: 0.0,
                meta: Meta::new()
                    .with_kind(AnalysisKind::ScriptAnalysis)
                    .with_text(position),
            }
        }

        fn number_substitution(position: u32, substitution: IDWriteNumberSubstitution) -> Self {
            Self {
                data: AnalysisData {
                    number: ManuallyDrop::new(substitution),
                },
                more: 0.0,
                meta: Meta::new()
                    .with_kind(AnalysisKind::NumberSubstitution)
                    .with_text(position),
            }
        }

        fn is_right_to_left(position: u32, is_rtl: bool) -> Self {
            Self {
                data: AnalysisData { is_rtl },
                more: 0.0,
                meta: Meta::new()
                    .with_kind(AnalysisKind::IsRightToLeft)
                    .with_text(position),
            }
        }
    }

    impl PartialEq for AnalysisResult {
        fn eq(&self, other: &Self) -> bool {
            self.meta.text() == other.meta.text()
        }
    }

    impl PartialOrd for AnalysisResult {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.meta.text().cmp(&other.meta.text()))
        }
    }

    impl Debug for AnalysisResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AnalysisResult")
                .field("text", &self.meta.text())
                .field("kind", &(self.meta.kind() as AnalysisKind))
                .field("data", &"{ packed }")
                .finish()
        }
    }

    struct AnalysisParams<'a> {
        text: &'a dyn TextView,
        style: TextStyle<'a>,
        font_set: &'a IDWriteFontCollection,
        fallback: &'a IDWriteFontFallback,
        numbers: &'a IDWriteNumberSubstitution,
        analyzer: &'a IDWriteTextAnalyzer,
    }

    impl<'a> AnalysisParams<'a> {
        fn run(self, arena: &Arena) -> Array<AnalysisResult> {
            let mut sink = Analysis::new(arena);

            let source = IDWriteTextAnalysisSource::from(WCharView {
                view: self.text,
                number_substitution: self.numbers,
            });

            // font fallback
            {
                let font = {
                    let mut buf: ArrayVec<u16, 256> = ArrayVec::new();
                    buf.extend(self.style.font_name.encode_utf16());
                    buf.push(0);
                    buf
                };
                let weight = translate_font_weights(self.style.font_weight);
                let style = translate_font_styles(self.style.font_style);

                let mut index = 0;

                let mut mapped_length = 0;
                let mut mapped_scale = 0.0;
                let mut mapped_font = None;

                while mapped_length < index {
                    unsafe {
                        self.fallback.MapCharacters(
                            &source,
                            index,
                            self.text.length() as u32,
                            self.font_set,
                            PCWSTR::from_raw(font.as_ptr()),
                            weight,
                            style,
                            DWRITE_FONT_STRETCH_UNDEFINED,
                            &mut mapped_length,
                            &mut mapped_font,
                            &mut mapped_scale,
                        )
                    }
                    .unwrap();

                    sink.results.push(AnalysisResult::backup_font(
                        index,
                        mapped_font.as_ref().unwrap().clone(),
                        1.0,
                    ));

                    index += mapped_length;
                }
            }

            let sink = IDWriteTextAnalysisSink::from(sink);

            unsafe {
                self.analyzer
                    .AnalyzeBidi(&source, 0, self.text.length() as u32, &sink);
            };

            sink.results
        }
    }

    #[implement(IDWriteTextAnalysisSink)]
    struct Analysis<'a> {
        results: Array<'a, AnalysisResult>,
    }

    impl<'a> Analysis<'a> {
        fn new(arena: &'a Arena) -> Self {
            let this = Self {
                results: Array::new_in(arena),
            };

            this
        }
    }

    impl IDWriteTextAnalysisSink_Impl for Analysis<'_> {
        fn SetScriptAnalysis(
            &self,
            textposition: u32,
            textlength: u32,
            scriptanalysis: *const DWRITE_SCRIPT_ANALYSIS,
        ) -> windows_core::Result<()> {
            todo!()
        }

        fn SetLineBreakpoints(
            &self,
            textposition: u32,
            textlength: u32,
            linebreakpoints: *const DWRITE_LINE_BREAKPOINT,
        ) -> windows_core::Result<()> {
            todo!()
        }

        fn SetBidiLevel(
            &self,
            textposition: u32,
            textlength: u32,
            explicitlevel: u8,
            resolvedlevel: u8,
        ) -> windows_core::Result<()> {
            todo!()
        }

        fn SetNumberSubstitution(
            &self,
            textposition: u32,
            textlength: u32,
            numbersubstitution: Option<&IDWriteNumberSubstitution>,
        ) -> windows_core::Result<()> {
            todo!()
        }
    }
}
