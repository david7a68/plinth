use dx12::{Dx12Buffer, Dx12Device, Dx12GraphicsCommandList, Dx12Image, Dx12Swapchain};
use windows::Win32::Foundation::HWND;

use self::dx12::Dx12Output;

use super::{GraphicsConfig, PresentStatistics, RefreshRate};

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
    Fixed { width: u32, height: u32 },
    Flex { width: u32, height: u32, flex: f32 },
}

pub(crate) struct Swapchain {
    swapchain: SwapchainImpl,
}

impl Swapchain {
    pub fn output(&self) -> &Output {
        match &self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.output(),
        }
    }

    pub fn present_statistics(&self) -> PresentStatistics {
        match &self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.present_statistics(),
        }
    }

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
    pub fn present(&mut self, submission_id: SubmissionId, intervals: u32) {
        match &mut self.swapchain {
            SwapchainImpl::Dx12(swapchain) => swapchain.present(submission_id, intervals),
        }
    }
}

enum SwapchainImpl {
    Dx12(Dx12Swapchain),
}

pub struct Output {
    #[cfg(target_os = "windows")]
    output: Dx12Output,
}

impl Output {
    pub fn refresh_rate(&self) -> RefreshRate {
        self.output.refresh_rate()
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_for_vsync(&self) {
        self.output.wait_for_vsync();
    }
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

pub(crate) struct Buffer {
    memory: BufferImpl,
}

impl Buffer {
    pub fn size(&self) -> u64 {
        match &self.memory {
            BufferImpl::Dx12(memory) => memory.size(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        match &mut self.memory {
            BufferImpl::Dx12(memory) => memory.as_mut_slice(),
        }
    }
}

enum BufferImpl {
    Dx12(Dx12Buffer),
}

pub(crate) struct Device {
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

    pub fn allocate_memory(&self, size: u64) -> Buffer {
        match &self.device {
            DeviceImpl::Dx12(device) => Buffer {
                memory: BufferImpl::Dx12(device.allocate_buffer(size)),
            },
        }
    }

    pub fn resize_memory(&self, memory: &mut Buffer, size: u64) {
        match &mut memory.memory {
            BufferImpl::Dx12(memory) => {
                let DeviceImpl::Dx12(device) = &self.device else {
                    panic!()
                };
                memory.resize(device, size);
            }
        }
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
