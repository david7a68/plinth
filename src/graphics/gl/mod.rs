use std::ops::{Deref, DerefMut};

use crate::{core::static_slot_map::new_key_type, system::WindowExtent};

use super::{
    Backend, DrawList, Format, FrameInfo, GraphicsConfig, Layout, RasterBuf,
    {TextureExtent, TexturePoint},
};

#[cfg(target_os = "windows")]
pub mod dx12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubmitId(pub(crate) u64);

new_key_type!(TextureId);

#[allow(clippy::large_enum_variant)]
pub enum Device {
    Null,
    Dx12(dx12::Device),
}

impl Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        match config.backend {
            Backend::Null => Device::Null,
            #[cfg(target_os = "windows")]
            Backend::Auto | Backend::Dx12 => Device::Dx12(dx12::Device::new(config)),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn create_swapchain(&self, hwnd: windows::Win32::Foundation::HWND) -> Swapchain {
        match self {
            Self::Null => Swapchain::Null,
            Self::Dx12(device) => Swapchain::Dx12(device.create_swapchain(hwnd)),
        }
    }

    pub fn create_texture(
        &self,
        extent: TextureExtent,
        layout: Layout,
        format: Format,
    ) -> TextureId {
        match self {
            Self::Null => TextureId::new(0, 0),
            Self::Dx12(device) => device.create_texture(extent, layout, format),
        }
    }

    pub fn copy_raster_to_texture(
        &self,
        target: TextureId,
        pixels: &RasterBuf,
        origin: TexturePoint,
    ) {
        match self {
            Self::Null => {}
            Self::Dx12(device) => device.copy_raster_to_texture(target, pixels, origin),
        }
    }

    pub fn flush_upload_buffer(&self) {
        match self {
            Self::Null => {}
            Self::Dx12(device) => device.flush_upload_buffer(),
        }
    }

    pub fn draw(&self, draw_list: &DrawList, target: &mut RenderTarget) {
        match (self, target) {
            (Self::Null, _) => {}
            (Self::Dx12(device), RenderTarget::Dx12(target)) => device.draw(draw_list, target),
            _ => panic!("Mismatched device and render target backends"),
        }
    }
}

pub enum Swapchain<'device> {
    Null,
    Dx12(dx12::Swapchain<'device>),
}

impl<'device> Swapchain<'device> {
    pub fn resize(&mut self, extent: WindowExtent) {
        match self {
            Self::Null => {}
            Self::Dx12(context) => context.resize(extent),
        }
    }

    pub fn next_image<'this>(&'this mut self) -> SwapchainImage<'this, 'device> {
        match self {
            Self::Null => SwapchainImage::Null(RenderTarget::Null),
            Self::Dx12(context) => SwapchainImage::Dx12(context.next_image()),
        }
    }
}

pub enum SwapchainImage<'a, 'b> {
    Null(RenderTarget),
    Dx12(dx12::SwapchainImage<'a, 'b>),
}

impl SwapchainImage<'_, '_> {
    pub fn frame_info(&self) -> FrameInfo {
        match self {
            Self::Null(_) => FrameInfo::default(),
            Self::Dx12(image) => image.frame_info(),
        }
    }

    pub fn present(self) {
        match self {
            Self::Null(_) => {}
            Self::Dx12(image) => image.present(),
        }
    }
}

impl Deref for SwapchainImage<'_, '_> {
    type Target = RenderTarget;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Null(target) => target,
            Self::Dx12(image) => image.render_target(),
        }
    }
}

impl DerefMut for SwapchainImage<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Null(target) => target,
            Self::Dx12(image) => image.render_target_mut(),
        }
    }
}

pub enum RenderTarget {
    Null,
    Dx12(dx12::RenderTarget),
}

impl RenderTarget {
    #[must_use]
    pub fn extent(&self) -> TextureExtent {
        match self {
            Self::Null => TextureExtent::ZERO,
            Self::Dx12(target) => target.extent(),
        }
    }
}
