use arrayvec::ArrayVec;

use windows::Win32::Graphics::Direct3D12::{
    ID3D12DescriptorHeap, ID3D12Device, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
    D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE,
    D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
    D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
    D3D12_GPU_DESCRIPTOR_HANDLE,
};

pub struct DescriptorArena {
    cpu_heap_start: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_heap_start: D3D12_GPU_DESCRIPTOR_HANDLE,
    handle_size: u32,
    allocated: u32,
    capacity: u32,

    #[allow(dead_code)]
    heap: ID3D12DescriptorHeap,
}

impl DescriptorArena {
    pub fn new(device: &ID3D12Device, kind: D3D12_DESCRIPTOR_HEAP_TYPE, capacity: u32) -> Self {
        let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: kind,
            NumDescriptors: capacity,
            Flags: kind_to_flag(kind),
            NodeMask: 0,
        };

        let heap: ID3D12DescriptorHeap = unsafe { device.CreateDescriptorHeap(&heap_desc) }
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create descriptor heap: {:?}", e);
                panic!()
            });

        Self {
            cpu_heap_start: unsafe { heap.GetCPUDescriptorHandleForHeapStart() },
            gpu_heap_start: unsafe { heap.GetGPUDescriptorHandleForHeapStart() },
            handle_size: unsafe { device.GetDescriptorHandleIncrementSize(kind) },
            allocated: 0,
            capacity,
            heap,
        }
    }

    pub fn reset(&mut self) {
        self.allocated = 0;
    }

    pub fn allocate(
        &mut self,
    ) -> Option<(D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_GPU_DESCRIPTOR_HANDLE)> {
        if self.allocated == self.capacity {
            return None;
        }

        let offset = self.allocated as usize * self.handle_size as usize;

        let cpu_handle = D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: self.cpu_heap_start.ptr + offset,
        };

        let gpu_handle = D3D12_GPU_DESCRIPTOR_HANDLE {
            ptr: self.gpu_heap_start.ptr + offset as u64,
        };

        self.allocated += 1;

        Some((cpu_handle, gpu_handle))
    }
}

pub struct SingleDescriptorHeap {
    cpu_heap_start: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_heap_start: D3D12_GPU_DESCRIPTOR_HANDLE,
    #[allow(dead_code)]
    heap: ID3D12DescriptorHeap,
}

impl SingleDescriptorHeap {
    pub fn new(device: &ID3D12Device, kind: D3D12_DESCRIPTOR_HEAP_TYPE) -> Self {
        let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: kind,
            NumDescriptors: 1,
            Flags: kind_to_flag(kind),
            NodeMask: 0,
        };

        let heap: ID3D12DescriptorHeap = unsafe { device.CreateDescriptorHeap(&heap_desc) }
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create descriptor heap: {:?}", e);
                panic!()
            });

        let cpu_heap_start = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };
        let gpu_heap_start = if is_shader_visible(kind) {
            unsafe { heap.GetGPUDescriptorHandleForHeapStart() }
        } else {
            D3D12_GPU_DESCRIPTOR_HANDLE { ptr: 0 }
        };

        Self {
            cpu_heap_start,
            gpu_heap_start,
            heap,
        }
    }

    pub fn cpu_handle(&self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        self.cpu_heap_start
    }

    pub fn gpu_handle(&self) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        self.gpu_heap_start
    }
}

fn is_shader_visible(kind: D3D12_DESCRIPTOR_HEAP_TYPE) -> bool {
    match kind {
        D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV => true,
        D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER => true,
        D3D12_DESCRIPTOR_HEAP_TYPE_RTV => false,
        D3D12_DESCRIPTOR_HEAP_TYPE_DSV => false,
        _ => false,
    }
}

fn kind_to_flag(kind: D3D12_DESCRIPTOR_HEAP_TYPE) -> D3D12_DESCRIPTOR_HEAP_FLAGS {
    if is_shader_visible(kind) {
        D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE
    } else {
        D3D12_DESCRIPTOR_HEAP_FLAG_NONE
    }
}
