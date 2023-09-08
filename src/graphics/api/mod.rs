use std::sync::Arc;

use euclid::Size2D;
use windows::Win32::Foundation::HWND;

use crate::shell::ScreenSpace;

use dx12::{Dx12Device, Dx12GraphicsCommandList, Dx12Image, Dx12Swapchain};

mod dx12;

// Implementation notes:
//
//  - The use of enums to represent backends is a compromise between safety and
//    performance. It allows us to avoid dynamic dispatch (and its associated
//    lookups) while preserving the ability to share the public API.
//    Furthermore, the use of an enum permits the compiler to eliminate
//    unnecessary branching if only one backend is available.
// - Unfortunately, this approach still has a per-call cost. However, the hope
//   (and indeed this is an unsubstantiated hope) is that the CPU branch
//   predictor can eliminate the cost in most cases.

pub struct GraphicsConfig {
    pub debug_mode: bool,
}

pub struct Image {
    image: ImageImpl,
}

#[doc(hidden)]
impl From<Dx12Image> for Image {
    fn from(image: Dx12Image) -> Self {
        Self {
            image: ImageImpl::Dx12(image),
        }
    }
}

#[doc(hidden)]
impl TryFrom<Image> for Dx12Image {
    type Error = ();

    fn try_from(image: Image) -> Result<Self, Self::Error> {
        match image.image {
            ImageImpl::Dx12(image) => Ok(image),
        }
    }
}

pub(super) enum ImageImpl {
    Dx12(Dx12Image),
}

#[derive(Debug)]
pub enum ResizeOp {
    Auto,
    Fixed {
        size: Size2D<u16, ScreenSpace>,
    },
    Flex {
        size: Size2D<u16, ScreenSpace>,
        flex: f32,
    },
}

pub struct Swapchain {
    device: Arc<DeviceImpl>,
    swapchain: SwapchainImpl,
}

impl Swapchain {
    pub(super) fn new(device: &Device, window: HWND) -> Self {
        let swapchain = match &*device.device {
            DeviceImpl::Dx12(device) => SwapchainImpl::Dx12(Dx12Swapchain::new(device, window)),
        };

        Self {
            device: device.device.clone(),
            swapchain,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn resize(&mut self, op: ResizeOp) {
        match &mut self.swapchain {
            SwapchainImpl::Dx12(swapchain) => {
                let DeviceImpl::Dx12(device) = &*self.device else {
                    panic!()
                };
                swapchain.resize(device, op);
            }
        }
    }

    pub fn get_back_buffer(&self) -> (&Image, u32) {
        match &self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.get_back_buffer(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn present(&mut self, submission_id: SubmissionId) {
        match &mut self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.present(submission_id),
        }
    }
}

enum SwapchainImpl {
    Dx12(Dx12Swapchain),
}

/// Uniquely identifies a submission to the GPU. Used to track when a submission
/// has completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubmissionId(pub(super) u64);

pub enum ResourceState {
    Present,
    RenderTarget,
}

pub struct GraphicsCommandList {
    cmd_list: GraphicsCommandListImpl,
}

impl GraphicsCommandList {
    pub(super) fn new(device: &Device) -> Self {
        let cmd_list = match &*device.device {
            DeviceImpl::Dx12(device) => {
                GraphicsCommandListImpl::Dx12(Dx12GraphicsCommandList::new(device))
            }
        };

        Self { cmd_list: cmd_list }
    }

    pub fn reset(&mut self) {
        match &mut self.cmd_list {
            GraphicsCommandListImpl::Dx12(cmd_list) => cmd_list.reset(),
        }
    }

    pub fn finish(&mut self) {
        match &mut self.cmd_list {
            GraphicsCommandListImpl::Dx12(cmd_list) => cmd_list.finish(),
        }
    }

    pub fn set_render_target(&mut self, target: &Image) {
        match &mut self.cmd_list {
            GraphicsCommandListImpl::Dx12(cmd_list) => cmd_list.set_render_target(target),
        }
    }

    pub fn clear(&mut self, color: [f32; 4]) {
        match &mut self.cmd_list {
            GraphicsCommandListImpl::Dx12(cmd_list) => cmd_list.clear(color),
        }
    }

    pub fn image_barrier(&mut self, image: &Image, from: ResourceState, to: ResourceState) {
        match &mut self.cmd_list {
            GraphicsCommandListImpl::Dx12(cmd_list) => cmd_list.image_barrier(image, from, to),
        }
    }
}

enum GraphicsCommandListImpl {
    Dx12(Dx12GraphicsCommandList),
}

pub struct Device {
    device: Arc<DeviceImpl>,
}

impl Device {
    pub(super) fn new(config: &GraphicsConfig) -> Self {
        let device = Arc::new(DeviceImpl::Dx12(Dx12Device::new(config)));

        Self { device }
    }

    pub fn wait_for_idle(&self) {
        match &*self.device {
            DeviceImpl::Dx12(device) => device.wait_for_idle(),
        }
    }

    pub fn most_recently_completed_submission(&self) -> SubmissionId {
        match &*self.device {
            DeviceImpl::Dx12(device) => device.most_recently_completed_submission(),
        }
    }

    #[tracing::instrument(skip(self, command_list))]
    pub fn submit_graphics_command_list(&self, command_list: &GraphicsCommandList) -> SubmissionId {
        match &*self.device {
            DeviceImpl::Dx12(device) => {
                let GraphicsCommandListImpl::Dx12(command_list) = &command_list.cmd_list else {
                    panic!("Invalid command list.");
                };

                device.submit_graphics_command_list(command_list)
            }
        }
    }
}

enum DeviceImpl {
    Dx12(Dx12Device),
}
