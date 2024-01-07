mod descriptor;
mod queue;
mod shaders;

use std::{mem::ManuallyDrop, sync::Arc};

use windows::{
    core::{ComInterface, PCSTR},
    Win32::{
        Foundation::RECT,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::{
                D3D12CreateDevice, ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device,
                ID3D12GraphicsCommandList, ID3D12InfoQueue1, ID3D12Resource,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
                D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN,
                D3D12_MESSAGE_CALLBACK_FLAG_NONE, D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID,
                D3D12_MESSAGE_SEVERITY, D3D12_MESSAGE_SEVERITY_CORRUPTION,
                D3D12_MESSAGE_SEVERITY_ERROR, D3D12_MESSAGE_SEVERITY_INFO,
                D3D12_MESSAGE_SEVERITY_MESSAGE, D3D12_MESSAGE_SEVERITY_WARNING,
                D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
                D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC,
                D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATES,
                D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_TRANSITION_BARRIER,
                D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_VIEWPORT,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
                IDXGIFactory2,
            },
        },
    },
};

use crate::graphics::GraphicsConfig;

use self::{descriptor::SingleDescriptorHeap, shaders::RectShader};

use super::gfx::{self, DrawCommand, DrawList, SubmitId};

const DEFAULT_DRAW_BUFFER_SIZE: u64 = 64 * 1024;
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
    buffer: Option<ID3D12Resource>,
    buffer_size: u64,
    buffer_ptr: *mut u8,

    submit_id: Option<SubmitId>,
    command_list: ID3D12GraphicsCommandList,
    command_list_mem: ID3D12CommandAllocator,
}

impl Frame {
    fn new(device: &ID3D12Device) -> Self {
        let buffer = alloc_buffer(device, DEFAULT_DRAW_BUFFER_SIZE);

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
            buffer: Some(buffer),
            buffer_size: DEFAULT_DRAW_BUFFER_SIZE,
            buffer_ptr,
            submit_id: None,
            command_list,
            command_list_mem: command_allocator,
        }
    }
}

impl gfx::Frame for Frame {}

pub struct Context {
    device: Arc<Device>,
    shaders: Shaders,
    rtv_heap: SingleDescriptorHeap,
}

impl Context {
    fn new(device: &Arc<Device>) -> Self {
        let rtv_heap = SingleDescriptorHeap::new(&device.device, D3D12_DESCRIPTOR_HEAP_TYPE_RTV);

        let shaders = Shaders::new(&device.device);

        Self {
            device: device.clone(),
            shaders,
            rtv_heap,
        }
    }
}

impl gfx::Context for Context {
    type Frame = Frame;
    type Image = Image;

    fn create_frame(&self) -> Self::Frame {
        Frame::new(&self.device.device)
    }

    #[tracing::instrument(skip(self, content, frame, target))]
    fn draw(
        &mut self,
        content: &DrawList,
        frame: &mut Self::Frame,
        target: impl Into<Self::Image>,
    ) -> gfx::SubmitId {
        if let Some(submit_id) = frame.submit_id {
            self.device.queue.wait(submit_id);
        }

        let target = target.into().image;

        let rects_size = std::mem::size_of_val(content.rects.as_slice());
        let buffer_size = rects_size as u64;

        if frame.buffer_size < buffer_size {
            let _ = frame.buffer.take();

            let buffer = alloc_buffer(&self.device.device, buffer_size);

            frame.buffer_ptr = {
                let mut mapped = std::ptr::null_mut();
                unsafe { buffer.Map(0, None, Some(&mut mapped)) }.unwrap();
                mapped.cast()
            };

            frame.buffer = Some(buffer);
            frame.buffer_size = buffer_size;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                content.rects.as_ptr().cast(),
                frame.buffer_ptr,
                rects_size,
            );
        }

        let target_rtv = self.rtv_heap.cpu_handle();

        unsafe {
            self.device
                .device
                .CreateRenderTargetView(&target, None, target_rtv);
        }

        let viewport_rect = {
            assert!(content.commands[0].0 == DrawCommand::Begin);
            let rect = content.areas[0];
            RECT {
                left: rect.left() as i32,
                top: rect.top() as i32,
                right: rect.right() as i32,
                bottom: rect.bottom() as i32,
            }
        };

        let viewport_scale = [
            1.0 / viewport_rect.right as f32,
            1.0 / viewport_rect.bottom as f32,
        ];

        let viewport = D3D12_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: viewport_rect.right as f32,
            Height: viewport_rect.bottom as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        let mut rect_idx = 0;
        let mut area_idx = 0;
        let mut color_idx = 0;

        for (command, idx) in &content.commands {
            match command {
                DrawCommand::Begin => unsafe {
                    frame.command_list_mem.Reset().unwrap();
                    frame
                        .command_list
                        .Reset(&frame.command_list_mem, None)
                        .unwrap();

                    image_barrier(
                        &frame.command_list,
                        &target,
                        D3D12_RESOURCE_STATE_PRESENT,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                    );

                    frame
                        .command_list
                        .OMSetRenderTargets(1, Some(&target_rtv), false, None);

                    frame.command_list.RSSetViewports(&[viewport]);
                    frame.command_list.RSSetScissorRects(&[viewport_rect]);
                },
                DrawCommand::End => unsafe {
                    image_barrier(
                        &frame.command_list,
                        &target,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                        D3D12_RESOURCE_STATE_PRESENT,
                    );
                    frame.command_list.Close().unwrap();
                },
                DrawCommand::Clip => {
                    let scissor = RECT {
                        left: content.areas[area_idx].left() as i32,
                        top: content.areas[area_idx].top() as i32,
                        right: content.areas[area_idx].right() as i32,
                        bottom: content.areas[area_idx].bottom() as i32,
                    };
                    unsafe { frame.command_list.RSSetScissorRects(&[scissor]) };
                    area_idx += 1;
                }
                DrawCommand::Clear => {
                    unsafe {
                        frame.command_list.ClearRenderTargetView(
                            target_rtv,
                            &content.clears[color_idx].to_array_f32(),
                            None,
                        );
                    }
                    color_idx += 1;
                }
                DrawCommand::DrawRects => {
                    // todo: suspect an off-by-one error here
                    let num_rects = *idx - rect_idx;

                    self.shaders.rect_shader.bind(
                        &frame.command_list,
                        frame.buffer.as_ref().unwrap(),
                        viewport_scale,
                        viewport.Height,
                    );

                    unsafe { frame.command_list.DrawInstanced(4, num_rects, 0, rect_idx) };

                    rect_idx = *idx;
                }
            }
        }

        let submit_id = self
            .device
            .queue
            .submit(&frame.command_list.cast().unwrap());

        frame.submit_id = Some(submit_id);

        submit_id
    }

    #[tracing::instrument(skip(self))]
    fn wait(&self, submit_id: gfx::SubmitId) {
        self.device.queue.wait(submit_id);
    }

    #[tracing::instrument(skip(self))]
    fn wait_for_idle(&self) {
        self.device.queue.wait_idle();
    }
}

pub struct Device {
    device: ID3D12Device,
    queue: queue::Queue,
}

impl Device {
    pub fn new(dxgi: &IDXGIFactory2, config: &GraphicsConfig) -> Arc<Self> {
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

        let queue = queue::Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

        Arc::new(Self { device, queue })
    }

    pub fn queue(&self) -> &ID3D12CommandQueue {
        &self.queue.queue
    }
}

impl gfx::Device for Arc<Device> {
    type Context = Context;

    fn create_context(&self) -> Self::Context {
        Context::new(self)
    }

    fn wait(&self, submit_id: gfx::SubmitId) {
        self.queue.wait(submit_id);
    }

    fn wait_for_idle(&self) {
        self.queue.wait_idle();
    }
}

struct Shaders {
    rect_shader: RectShader,
}

impl Shaders {
    pub fn new(device: &ID3D12Device) -> Self {
        let rect_shader = RectShader::new(device);

        Self { rect_shader }
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

pub fn alloc_buffer(device: &ID3D12Device, size: u64) -> ID3D12Resource {
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
