mod canvas;
mod context;
mod device;
mod shaders;

pub use canvas::Canvas;
pub use context::Context;

use windows::Win32::{
    Foundation::HWND,
    Graphics::DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
};

use crate::graphics::GraphicsConfig;

pub struct Graphics {
    device: device::Device,
    compositor: IDCompositionDevice,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = device::Device::new(config);
        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();
        Self { compositor, device }
    }

    pub fn create_context(&self, hwnd: HWND) -> Context {
        Context::new(&self.device, &self.compositor, hwnd)
    }
}
