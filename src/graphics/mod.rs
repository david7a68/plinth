pub(crate) mod backend;
mod canvas;
mod color;
mod frame_statistics;
mod primitives;

use windows::Win32::Foundation::HWND;

use self::backend::{Device, ResizeOp, SubmissionId, Swapchain};
pub use self::canvas::*;
pub use self::color::*;
pub use self::frame_statistics::*;
pub use self::primitives::*;

pub enum PowerPreference {
    LowPower,
    HighPerformance,
}

pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::HighPerformance,
            debug_mode: cfg!(debug_assertions),
        }
    }
}

pub(crate) struct Graphics {
    device: backend::Device,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        Self {
            device: Device::new(config),
        }
    }

    pub fn create_swapchain(&self, window: HWND) -> Swapchain {
        self.device.create_swapchain(window)
    }

    pub fn resize_swapchain(&self, swapchain: &mut Swapchain, op: ResizeOp) {
        self.device.resize_swapchain(swapchain, op);
    }

    pub fn create_draw_buffer(&self) -> DrawData {
        let command_list = self.device.create_graphics_command_list();
        DrawData::new(command_list)
    }

    pub fn draw(&self, buffer: &DrawData) -> SubmissionId {
        self.device
            .submit_graphics_command_list(&buffer.command_list)
    }

    pub fn wait_for_submission(&self, submission_id: SubmissionId) {
        self.device.wait_for_submission(submission_id);
    }

    pub fn wait_for_idle(&self) {
        self.device.wait_for_idle();
    }
}
