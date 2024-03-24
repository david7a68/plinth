use crate::{
    core::static_slot_map::new_key_type,
    geometry::{Extent, Pixel, Point, Scale, Texel, Wixel},
};

use self::draw_list::Canvas;

use super::{Backend, Format, FrameInfo, GraphicsConfig, Layout, PixelBuf};

#[cfg(target_os = "windows")]
pub mod dx12;

pub mod draw_list;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubmitId(pub(crate) u64);

new_key_type!(TextureId);

pub enum Device {
    Null,
    Dx12(dx12::Device),
}

impl Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        match config.backend {
            #[cfg(target_os = "windows")]
            Backend::Auto | Backend::Dx12 => Device::Dx12(dx12::Device::new(config)),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn create_context(&self, hwnd: windows::Win32::Foundation::HWND) -> WindowContext {
        match self {
            Self::Null => WindowContext::Null,
            Self::Dx12(device) => WindowContext::Dx12(device.create_context(hwnd)),
        }
    }

    pub fn create_texture(
        &self,
        extent: Extent<Texel>,
        layout: Layout,
        format: Format,
    ) -> TextureId {
        match self {
            Self::Null => TextureId::new(0, 0),
            Self::Dx12(device) => device.create_texture(extent, layout, format),
        }
    }

    pub fn create_uploader(&self) -> Uploader {
        match self {
            Self::Null => Uploader::Null,
            Self::Dx12(device) => Uploader::Dx12(device.create_uploader()),
        }
    }
}

pub enum Uploader {
    Null,
    Dx12(dx12::Uploader),
}

impl Uploader {
    pub fn upload_image(&mut self, target: TextureId, pixels: &PixelBuf, origin: Point<Texel>) {
        match self {
            Self::Null => {}
            Self::Dx12(uploader) => uploader.upload_image(target, pixels, origin),
        }
    }

    pub fn flush_upload_buffer(&mut self) {
        match self {
            Self::Null => {}
            Self::Dx12(uploader) => uploader.flush_upload_buffer(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum WindowContext<'a> {
    Null,
    Dx12(dx12::WindowContext<'a>),
}

impl WindowContext<'_> {
    pub fn resize(&mut self, extent: Extent<Wixel>) {
        match self {
            Self::Null => {}
            Self::Dx12(context) => context.resize(extent),
        }
    }

    pub fn change_dpi(&mut self, size: Extent<Wixel>, scale: Scale<Wixel, Pixel>) {
        match self {
            Self::Null => {}
            Self::Dx12(context) => context.change_dpi(size, scale),
        }
    }

    pub fn draw(&mut self, callback: impl FnMut(&mut Canvas, &FrameInfo)) {
        match self {
            Self::Null => {}
            Self::Dx12(context) => context.draw(callback),
        }
    }
}
