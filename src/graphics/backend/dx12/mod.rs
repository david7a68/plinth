mod canvas;
mod context;
mod device;
mod shaders;

use std::sync::Arc;

pub use canvas::Canvas;
pub use context::Context;

use windows::Win32::{
    Foundation::HWND,
    Graphics::DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
};

use crate::graphics::GraphicsConfig;

pub struct Graphics {
    device: Arc<device::Device>,
    compositor: IDCompositionDevice,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = Arc::new(device::Device::new(config));
        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();
        Self { device, compositor }
    }

    pub fn create_context(&self, hwnd: HWND) -> Context {
        Context::new(self.device.clone(), &self.compositor, hwnd)
    }
}
