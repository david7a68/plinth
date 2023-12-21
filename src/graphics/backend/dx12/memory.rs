use std::{
    ffi::c_void,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use windows::Win32::Graphics::{
    Direct3D12::{
        ID3D12Resource, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_HEAP_FLAG_NONE,
        D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN,
        D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE,
        D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
    },
    Dxgi::Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
};

use super::Dx12Device;

pub(crate) struct Dx12Buffer {
    // Use this instead of querying the buffer to avoid a virtual call.
    size: u64,

    // We currently assume that we're only working with upload buffers. This may
    // (probably will) change in the future.
    mapped: NonNull<c_void>,

    // This allows us to free the resource before allocating a new one to
    // replace it.
    buffer: MaybeUninit<ID3D12Resource>,
}

impl Dx12Buffer {
    pub fn new(device: &Dx12Device, size: u64) -> Self {
        let buffer = alloc_buffer(device, size);

        let mapped = {
            let mut mapped = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
            NonNull::new(mapped).unwrap()
        };

        Self {
            size,
            mapped,
            buffer: MaybeUninit::new(buffer),
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn resize(&mut self, device: &Dx12Device, size: u64) {
        unsafe {
            self.buffer.assume_init_ref().Unmap(0, None);
            self.buffer.assume_init_drop();
        }

        let buffer = alloc_buffer(device, size);

        self.mapped = {
            let mut mapped = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
            NonNull::new(mapped).unwrap()
        };

        self.buffer = MaybeUninit::new(buffer);
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mapped.as_ptr() as *const u8, self.size as usize) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.mapped.as_ptr() as *mut u8, self.size as usize)
        }
    }
}

impl Drop for Dx12Buffer {
    fn drop(&mut self) {
        unsafe { self.buffer.assume_init_drop() };
    }
}

fn alloc_buffer(device: &Dx12Device, size: u64) -> ID3D12Resource {
    let heap_desc = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_UPLOAD,
        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
        CreationNodeMask: 0,
        VisibleNodeMask: 0,
    };

    let buffer_desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Alignment: 0,
        Width: size,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: DXGI_FORMAT_UNKNOWN,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: D3D12_RESOURCE_FLAG_NONE,
    };

    let mut buffer = None;

    unsafe {
        device.device.CreateCommittedResource(
            &heap_desc,
            D3D12_HEAP_FLAG_NONE,
            &buffer_desc,
            D3D12_RESOURCE_STATE_GENERIC_READ,
            None,
            &mut buffer,
        )
    }
    .unwrap();

    buffer.unwrap()
}
