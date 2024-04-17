mod device;
mod shaders;
mod swapchain;
mod uploader;

use std::mem::ManuallyDrop;

use windows::Win32::Graphics::{
    Direct3D12::{
        ID3D12GraphicsCommandList, ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE,
        D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        D3D12_RESOURCE_STATES, D3D12_RESOURCE_TRANSITION_BARRIER,
    },
    Dxgi::Common::{
        DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R8_UNORM,
    },
};

use crate::graphics::{Format, Layout, TextureExtent};

pub use self::device::Device;
pub use swapchain::{Swapchain, SwapchainImage};

use super::SubmitId;

pub struct RenderTarget {
    pub draw: Option<SubmitId>,
    pub size: TextureExtent,
    pub state: D3D12_RESOURCE_STATES,
    pub resource: ID3D12Resource,
    pub descriptor: D3D12_CPU_DESCRIPTOR_HANDLE,
}

impl RenderTarget {
    pub fn extent(&self) -> TextureExtent {
        self.size
    }
}

pub fn image_barrier(
    command_list: &ID3D12GraphicsCommandList,
    image: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
    to: D3D12_RESOURCE_STATES,
) {
    let transition = D3D12_RESOURCE_TRANSITION_BARRIER {
        pResource: unsafe { std::mem::transmute_copy(image) },
        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        StateBefore: from,
        StateAfter: to,
    };

    let barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(transition),
        },
    };

    unsafe { command_list.ResourceBarrier(&[barrier]) };
}

fn to_dxgi_format(layout: Layout, format: Format) -> DXGI_FORMAT {
    #[allow(clippy::match_same_arms)]
    match (layout, format) {
        (_, Format::Unkown) => panic!("Unknown format"),
        (Layout::Rgba8, Format::Srgb) => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        (Layout::Rgba8, Format::Linear) => DXGI_FORMAT_R8G8B8A8_UNORM,
        (Layout::Rgba8Vector, Format::Srgb) => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        (Layout::Rgba8Vector, Format::Linear) => DXGI_FORMAT_R8G8B8A8_UNORM,
        (Layout::Bgra8, Format::Srgb) => DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        (Layout::Bgra8, Format::Linear) => DXGI_FORMAT_B8G8R8A8_UNORM,
        (Layout::Alpha8, Format::Linear) => DXGI_FORMAT_R8_UNORM,
        (Layout::Alpha8, Format::Srgb) => panic!("Alpha8 is not supported in SRGB format"),
        (Layout::Alpha8Vector, Format::Srgb) => {
            panic!("Alpha8Vector is not supported in SRGB format")
        }
        (Layout::Alpha8Vector, Format::Linear) => DXGI_FORMAT_R8_UNORM,
    }
}
