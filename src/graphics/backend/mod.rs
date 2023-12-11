use dx12::{Dx12Device, Dx12GraphicsCommandList, Dx12Image, Dx12Swapchain};
use windows::Win32::Foundation::HWND;

use super::GraphicsConfig;

mod dx12;
mod dxgi;

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

#[derive(Clone, Copy, Debug)]
enum Error {
    ObjectDestroyed,
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
    Fixed { width: u32, height: u32 },
    Flex { width: u32, height: u32, flex: f32 },
}

pub struct Swapchain {
    swapchain: SwapchainImpl,
}

impl Swapchain {
    fn resize(&mut self, device: &Device, op: ResizeOp) {
        match &mut self.swapchain {
            SwapchainImpl::Dx12(swapchain) => {
                let DeviceImpl::Dx12(device) = &device.device else {
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

    pub fn wait_for_vsync(&self) {
        match &self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.wait_for_vsync(),
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
    fn new(device: &Device) -> Self {
        let cmd_list = match &device.device {
            DeviceImpl::Dx12(device) => {
                GraphicsCommandListImpl::Dx12(Dx12GraphicsCommandList::new(device))
            }
        };

        Self { cmd_list }
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

struct Submit {
    id: SubmissionId,
    command_list: GraphicsCommandList,
}

pub struct Device {
    device: DeviceImpl,
}

impl Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = DeviceImpl::Dx12(Dx12Device::new(config));

        Self { device }
    }

    pub fn create_swapchain(&self, window: HWND) -> Swapchain {
        match &self.device {
            DeviceImpl::Dx12(device) => Swapchain {
                swapchain: SwapchainImpl::Dx12(device.create_swapchain(window)),
            },
        }
    }

    pub fn resize_swapchain(&self, swapchain: &mut Swapchain, op: ResizeOp) {
        swapchain.resize(self, op);
    }

    pub fn create_graphics_command_list(&self) -> GraphicsCommandList {
        GraphicsCommandList::new(self)
    }

    pub fn wait_for_idle(&self) {
        match &self.device {
            DeviceImpl::Dx12(device) => device.wait_for_idle(),
        }
    }

    pub fn wait_for_submission(&self, submission_id: SubmissionId) {
        match &self.device {
            DeviceImpl::Dx12(device) => device.wait_for_submission(submission_id),
        }
    }

    pub fn most_recently_completed_submission(&self) -> SubmissionId {
        match &self.device {
            DeviceImpl::Dx12(device) => device.most_recently_completed_submission(),
        }
    }

    #[tracing::instrument(skip(self, command_list))]
    pub fn submit_graphics_command_list(&self, command_list: &GraphicsCommandList) -> SubmissionId {
        match &self.device {
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
