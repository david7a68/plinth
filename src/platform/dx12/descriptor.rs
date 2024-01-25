use windows::Win32::Graphics::Direct3D12::{
    ID3D12DescriptorHeap, ID3D12Device, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
    D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE,
    D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
};

pub struct SingleDescriptorHeap {
    cpu_heap_start: D3D12_CPU_DESCRIPTOR_HANDLE,
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

        Self {
            cpu_heap_start,
            heap,
        }
    }

    pub fn cpu_handle(&self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        self.cpu_heap_start
    }
}

fn is_shader_visible(kind: D3D12_DESCRIPTOR_HEAP_TYPE) -> bool {
    matches!(
        kind,
        D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV | D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER
    )
}

fn kind_to_flag(kind: D3D12_DESCRIPTOR_HEAP_TYPE) -> D3D12_DESCRIPTOR_HEAP_FLAGS {
    if is_shader_visible(kind) {
        D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE
    } else {
        D3D12_DESCRIPTOR_HEAP_FLAG_NONE
    }
}
