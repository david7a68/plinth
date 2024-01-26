mod descriptor;
pub mod dxwindow;
mod queue;
mod shaders;

use std::{mem::ManuallyDrop, sync::Arc};

use windows::{
    core::{ComInterface, PCSTR},
    Win32::Graphics::{
        Direct3D::D3D_FEATURE_LEVEL_12_0,
        Direct3D12::{
            D3D12CreateDevice, ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device,
            ID3D12GraphicsCommandList, ID3D12InfoQueue1, ID3D12Resource,
            D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_HEAP_FLAG_NONE,
            D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN,
            D3D12_MESSAGE_CALLBACK_FLAG_NONE, D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID,
            D3D12_MESSAGE_SEVERITY, D3D12_MESSAGE_SEVERITY_CORRUPTION,
            D3D12_MESSAGE_SEVERITY_ERROR, D3D12_MESSAGE_SEVERITY_INFO,
            D3D12_MESSAGE_SEVERITY_MESSAGE, D3D12_MESSAGE_SEVERITY_WARNING, D3D12_RESOURCE_BARRIER,
            D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE,
            D3D12_RESOURCE_STATES, D3D12_RESOURCE_STATE_GENERIC_READ,
            D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        },
        Dxgi::{
            Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
            IDXGIFactory2,
        },
    },
};

use crate::graphics::GraphicsConfig;

use self::{queue::SubmitId, shaders::RectShader};

const DEFAULT_DRAW_BUFFER_SIZE: u64 = 64 * 1024;

pub struct Frame {
    buffer: Option<ID3D12Resource>,
    buffer_size: u64,
    buffer_ptr: *mut u8,

    submit_id: Option<SubmitId>,
    command_list: ID3D12GraphicsCommandList,
    command_list_mem: ID3D12CommandAllocator,
}

impl Frame {
    fn new(device: &Device) -> Self {
        let buffer = device.alloc_buffer(DEFAULT_DRAW_BUFFER_SIZE);

        let buffer_ptr = {
            let mut mapped = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
            mapped.cast()
        };

        let command_allocator: ID3D12CommandAllocator = unsafe {
            device
                .inner
                .device
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        }
        .unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.inner.device.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &command_allocator,
                None,
            )
        }
        .unwrap();

        unsafe { command_list.Close() }.unwrap();

        Self {
            buffer: Some(buffer),
            buffer_size: DEFAULT_DRAW_BUFFER_SIZE,
            buffer_ptr,
            submit_id: None,
            command_list,
            command_list_mem: command_allocator,
        }
    }
}

#[derive(Clone)]
pub struct Device {
    inner: Arc<Device_>,
}

impl Device {
    pub fn new(dxgi: &IDXGIFactory2, config: &GraphicsConfig) -> Self {
        let inner = {
            let device = {
                let adapter = unsafe { dxgi.EnumAdapters1(0) }.unwrap();

                let mut device: Option<ID3D12Device> = None;
                unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut device) }
                    .unwrap();

                device.unwrap()
            };

            if config.debug_mode {
                if let Ok(info_queue) = device.cast::<ID3D12InfoQueue1>() {
                    let mut cookie = 0;
                    unsafe {
                        info_queue.RegisterMessageCallback(
                            Some(dx12_debug_callback),
                            D3D12_MESSAGE_CALLBACK_FLAG_NONE,
                            std::ptr::null(),
                            &mut cookie,
                        )
                    }
                    .unwrap();
                }
            }

            let queue = queue::Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

            let rect_shader = RectShader::new(&device);

            Device_ {
                device,
                queue,
                rect_shader,
            }
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn queue(&self) -> &ID3D12CommandQueue {
        &self.inner.queue.handle
    }

    pub fn wait(&self, submit_id: SubmitId) {
        self.inner.queue.wait(submit_id);
    }

    pub fn wait_for_idle(&self) {
        self.inner.queue.wait_idle();
    }

    pub fn alloc_buffer(&self, size: u64) -> ID3D12Resource {
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
            self.inner.device.CreateCommittedResource(
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

    pub fn submit_graphics(&self, command_list: &ID3D12GraphicsCommandList) -> SubmitId {
        self.inner.queue.submit(&command_list.cast().unwrap())
    }
}

struct Device_ {
    device: ID3D12Device,
    queue: queue::Queue,
    rect_shader: RectShader,
}

unsafe extern "system" fn dx12_debug_callback(
    _category: D3D12_MESSAGE_CATEGORY,
    severity: D3D12_MESSAGE_SEVERITY,
    _id: D3D12_MESSAGE_ID,
    description: PCSTR,
    _context: *mut std::ffi::c_void,
) {
    match severity {
        D3D12_MESSAGE_SEVERITY_CORRUPTION => tracing::error!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_ERROR => tracing::error!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_WARNING => tracing::warn!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_INFO => tracing::debug!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_MESSAGE => tracing::info!("D3D12 {}", description.display()),
        _ => {}
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
