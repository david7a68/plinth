use windows::{
    core::w,
    Win32::Graphics::Direct3D12::{
        ID3D12CommandAllocator, ID3D12GraphicsCommandList, D3D12_COMMAND_LIST_TYPE_DIRECT,
        D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
        D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
        D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
        D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
        D3D12_RESOURCE_TRANSITION_BARRIER,
    },
};

use crate::graphics::backend::{Image, ResourceState};

use super::{Dx12Device, Dx12Image};

pub(crate) struct Dx12GraphicsCommandList {
    pub command_list: ID3D12GraphicsCommandList,
    pub command_allocator: ID3D12CommandAllocator,

    render_target: Option<D3D12_CPU_DESCRIPTOR_HANDLE>,
}

impl Dx12GraphicsCommandList {
    pub fn new(device: &Dx12Device) -> Self {
        let command_allocator = unsafe {
            device
                .device
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        }
        .unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.device.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &command_allocator,
                None,
            )
        }
        .unwrap();

        unsafe {
            command_list.Close().unwrap();
            command_list.SetName(w!("Graphics Command List")).unwrap();
        }

        Self {
            command_list,
            command_allocator,
            render_target: None,
        }
    }

    pub fn reset(&mut self) {
        self.render_target = None;
        unsafe { self.command_allocator.Reset() }.unwrap();
        unsafe { self.command_list.Reset(&self.command_allocator, None) }.unwrap();
    }

    pub fn finish(&mut self) {
        unsafe { self.command_list.Close() }.unwrap();
    }

    pub fn set_render_target(&mut self, image: &Image) {
        let image: &Dx12Image = image.try_into().unwrap();
        self.render_target = Some(image.render_target_view);

        unsafe {
            self.command_list
                .OMSetRenderTargets(1, Some(&image.render_target_view), false, None);
        }
    }

    pub fn clear(&mut self, color: [f32; 4]) {
        let render_target = self.render_target.unwrap();

        unsafe {
            self.command_list
                .ClearRenderTargetView(render_target, &color, None)
        };
    }

    pub fn image_barrier(&mut self, image: &Image, from: ResourceState, to: ResourceState) {
        fn translate(state: ResourceState) -> D3D12_RESOURCE_STATES {
            match state {
                ResourceState::Present => D3D12_RESOURCE_STATE_PRESENT,
                ResourceState::RenderTarget => D3D12_RESOURCE_STATE_RENDER_TARGET,
            }
        }

        let image: &Dx12Image = image.try_into().unwrap();
        let transition = D3D12_RESOURCE_TRANSITION_BARRIER {
            pResource: unsafe { std::mem::transmute_copy(&image.handle) },
            Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            StateBefore: translate(from),
            StateAfter: translate(to),
        };
        let barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: std::mem::ManuallyDrop::new(transition),
            },
        };

        unsafe { self.command_list.ResourceBarrier(&[barrier]) };
    }
}
