mod descriptor;
mod queue;

use std::{
    mem::{ManuallyDrop, MaybeUninit},
    sync::Arc,
};

use parking_lot::Mutex;
use windows::{
    core::{ComInterface, PCSTR},
    Win32::{
        Foundation::RECT,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::{
                D3D12CreateDevice, ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device,
                ID3D12GraphicsCommandList, ID3D12InfoQueue1, ID3D12Resource,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_UPLOAD,
                D3D12_MEMORY_POOL_UNKNOWN, D3D12_MESSAGE_CALLBACK_FLAG_NONE,
                D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID, D3D12_MESSAGE_SEVERITY,
                D3D12_MESSAGE_SEVERITY_CORRUPTION, D3D12_MESSAGE_SEVERITY_ERROR,
                D3D12_MESSAGE_SEVERITY_INFO, D3D12_MESSAGE_SEVERITY_MESSAGE,
                D3D12_MESSAGE_SEVERITY_WARNING, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
                D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC,
                D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATES,
                D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_TRANSITION_BARRIER,
                D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
                IDXGIFactory2,
            },
        },
    },
};

use crate::graphics::GraphicsConfig;

use self::descriptor::SimpleDescriptorHeap;

use super::gfx::{self, DrawCommand, DrawList};

const DEFAULT_DRAW_BUFFER_SIZE: u64 = 64 * 1024;
const MAX_RENDER_TARGET_VIEWS: usize = 128; // 128 / 2 = 64 windows

pub struct Image {
    image: ID3D12Resource,
}

impl gfx::Image for Image {}

impl From<ID3D12Resource> for Image {
    fn from(image: ID3D12Resource) -> Self {
        Self { image }
    }
}

pub struct Frame {
    device: ID3D12Device,
    rtv_heap: Arc<Mutex<SimpleDescriptorHeap<MAX_RENDER_TARGET_VIEWS>>>,

    target_rtv: D3D12_CPU_DESCRIPTOR_HANDLE,

    buffer: MaybeUninit<ID3D12Resource>,
    buffer_size: u64,
    buffer_ptr: *mut u8,

    command_list: ID3D12GraphicsCommandList,
    command_list_mem: ID3D12CommandAllocator,
}

impl Frame {
    fn new(
        device: &ID3D12Device,
        rtv_heap: Arc<Mutex<SimpleDescriptorHeap<MAX_RENDER_TARGET_VIEWS>>>,
    ) -> Self {
        let target_rtv = rtv_heap.lock().allocate().unwrap();

        let buffer = Self::alloc_buffer(device, DEFAULT_DRAW_BUFFER_SIZE);

        let buffer_ptr = {
            let mut mapped = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
            mapped.cast()
        };

        let command_allocator: ID3D12CommandAllocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
        }
        .unwrap();

        unsafe { command_list.Close() }.unwrap();

        Self {
            device: device.clone(),
            rtv_heap,
            target_rtv,
            buffer: MaybeUninit::new(buffer),
            buffer_size: DEFAULT_DRAW_BUFFER_SIZE,
            buffer_ptr,
            command_list,
            command_list_mem: command_allocator,
        }
    }

    fn alloc_buffer(device: &ID3D12Device, size: u64) -> ID3D12Resource {
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
            device.CreateCommittedResource(
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

    fn image_barrier(
        &mut self,
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

        unsafe { self.command_list.ResourceBarrier(&[barrier]) };
    }

    fn upload_draw_list(&mut self, draw_list: &gfx::DrawList, target: &ID3D12Resource) {
        let rect_size = std::mem::size_of_val(draw_list.rects.as_slice());
        let buffer_size = rect_size as u64;

        if self.buffer_size < buffer_size {
            unsafe {
                self.buffer.assume_init_ref().Unmap(0, None);
                self.buffer.assume_init_drop();
            }

            let buffer = Self::alloc_buffer(&self.device, buffer_size);

            self.buffer_ptr = {
                let mut mapped = std::ptr::null_mut();
                unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
                mapped.cast()
            };

            self.buffer = MaybeUninit::new(buffer);
            self.buffer_size = buffer_size;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                draw_list.rects.as_ptr().cast(),
                self.buffer_ptr,
                rect_size,
            );
        }

        // supposedly, you only need a single rtv which can be shared by
        // everyone since RTVs get written directly into the command list. Could
        // share one per thread if parallelism is a concern.
        //
        // This way is simpler, but needs a large descriptor heap.
        //
        // cleanup: single rtv per thread (need to set max window limit)
        unsafe {
            self.device
                .CreateRenderTargetView(target, None, self.target_rtv);
        }

        let mut rect_idx = 0;
        let mut area_idx = 0;
        let mut color_idx = 0;

        for (command, idx) in &draw_list.commands {
            match command {
                DrawCommand::Begin => unsafe {
                    self.command_list_mem.Reset().unwrap();
                    self.command_list
                        .Reset(&self.command_list_mem, None)
                        .unwrap();

                    self.image_barrier(
                        target,
                        D3D12_RESOURCE_STATE_PRESENT,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                    );

                    self.command_list
                        .OMSetRenderTargets(1, Some(&self.target_rtv), false, None);
                },
                DrawCommand::End => unsafe {
                    self.image_barrier(
                        target,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                        D3D12_RESOURCE_STATE_PRESENT,
                    );
                    self.command_list.Close().unwrap();
                },
                DrawCommand::Clip => {
                    let scissor = RECT {
                        left: draw_list.areas[area_idx].left() as i32,
                        top: draw_list.areas[area_idx].top() as i32,
                        right: draw_list.areas[area_idx].right() as i32,
                        bottom: draw_list.areas[area_idx].bottom() as i32,
                    };
                    unsafe { self.command_list.RSSetScissorRects(&[scissor]) };
                    area_idx += 1;
                }
                DrawCommand::Clear => {
                    unsafe {
                        self.command_list.ClearRenderTargetView(
                            self.target_rtv,
                            &draw_list.clears[color_idx].to_array_f32(),
                            None,
                        );
                    }
                    color_idx += 1;
                }
                DrawCommand::DrawRects => {
                    // todo: suspect an off-by-one error here
                    let _num_rects = *idx - rect_idx;
                    rect_idx = *idx;
                }
            }
        }
    }
}

impl gfx::Frame for Frame {}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { self.buffer.assume_init_drop() };
        self.rtv_heap.lock().deallocate(self.target_rtv);
    }
}

pub struct Device {
    device: ID3D12Device,
    queue: queue::Queue,
    rtv_heap: Arc<Mutex<descriptor::SimpleDescriptorHeap<MAX_RENDER_TARGET_VIEWS>>>,
}

impl Device {
    pub fn new(dxgi: &IDXGIFactory2, config: &GraphicsConfig) -> Self {
        let device = {
            let adapter = unsafe { dxgi.EnumAdapters1(0) }.unwrap();

            let mut device: Option<ID3D12Device> = None;
            unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut device) }.unwrap();

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

        let rtv_heap = Arc::new(Mutex::new(descriptor::SimpleDescriptorHeap::new(
            &device,
            D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            false,
        )));

        let queue = queue::Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

        Self {
            device,
            queue,
            rtv_heap,
        }
    }

    pub fn queue(&self) -> &ID3D12CommandQueue {
        &self.queue.queue
    }
}

impl gfx::Device for Device {
    type Frame = Frame;
    type Image = Image;

    fn create_frame(&self) -> Self::Frame {
        Frame::new(&self.device, self.rtv_heap.clone())
    }

    fn draw(
        &self,
        content: &DrawList,
        frame: &mut Self::Frame,
        target: impl Into<Self::Image>,
    ) -> gfx::SubmitId {
        let target = target.into();
        frame.upload_draw_list(content, &target.image);
        self.queue.submit(&frame.command_list.cast().unwrap())
    }

    fn wait(&self, submit_id: gfx::SubmitId) {
        self.queue.wait(submit_id);
    }

    fn wait_for_idle(&self) {
        self.queue.wait_idle();
    }
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
