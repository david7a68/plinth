use std::{
    cell::{Ref, RefCell},
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};

use windows::{
    core::{Interface, PCSTR},
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandAllocator,
                ID3D12CommandList, ID3D12CommandQueue, ID3D12Debug1, ID3D12Debug5,
                ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList,
                ID3D12InfoQueue1, ID3D12Resource, D3D12_COMMAND_LIST_TYPE,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DESCRIPTOR_HEAP_DESC,
                D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                D3D12_DESCRIPTOR_HEAP_TYPE, D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                D3D12_FENCE_FLAG_NONE, D3D12_GPU_DESCRIPTOR_HANDLE, D3D12_HEAP_FLAG_NONE,
                D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_UPLOAD,
                D3D12_MEMORY_POOL_UNKNOWN, D3D12_MESSAGE_CALLBACK_FLAG_NONE,
                D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID, D3D12_MESSAGE_SEVERITY,
                D3D12_MESSAGE_SEVERITY_CORRUPTION, D3D12_MESSAGE_SEVERITY_ERROR,
                D3D12_MESSAGE_SEVERITY_INFO, D3D12_MESSAGE_SEVERITY_MESSAGE,
                D3D12_MESSAGE_SEVERITY_WARNING, D3D12_RESOURCE_DESC,
                D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_GENERIC_READ,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_TEXTURE_LAYOUT_UNKNOWN, D3D12_VIEWPORT,
            },
            DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
            Dxgi::{
                Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
                CreateDXGIFactory2, IDXGIFactory2, DXGI_CREATE_FACTORY_DEBUG,
            },
        },
    },
};

use crate::{
    core::static_slot_map::SlotMap,
    graphics::{
        draw_list::Command,
        gl::{dx12::image_barrier, SubmitId, TextureId},
        DrawList, Format, GraphicsConfig, Layout, RasterBuf, TextureExtent, TexturePoint,
    },
};

use super::{shaders::RectShader, to_dxgi_format, uploader::Uploader, RenderTarget, Swapchain};

const DEFAULT_DRAW_BUFFER_SIZE: u64 = 64 * 1024;

pub struct Device {
    pub dxgi: IDXGIFactory2,
    pub handle: ID3D12Device,
    pub queue: Queue,
    pub rect_shader: RectShader,
    pub compositor: IDCompositionDevice,

    uploader: RefCell<Uploader>,

    command_list: ID3D12GraphicsCommandList,
    frames: RefCell<VecDeque<Frame>>,

    textures: RefCell<Box<SlotMap<1024, ID3D12Resource, TextureId>>>,
    pub texture_descriptors: DescriptorHeap,
}

impl Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        let dxgi: IDXGIFactory2 = {
            let mut dxgi_flags = 0;

            if config.debug_mode {
                let mut controller: Option<ID3D12Debug1> = None;
                unsafe { D3D12GetDebugInterface(&mut controller) }.unwrap();

                if let Some(controller) = controller {
                    eprintln!("Enabling D3D12 debug layer");
                    unsafe { controller.EnableDebugLayer() };
                    unsafe { controller.SetEnableGPUBasedValidation(true) };

                    if let Ok(controller) = controller.cast::<ID3D12Debug5>() {
                        unsafe { controller.SetEnableAutoName(true) };
                    }
                } else {
                    eprintln!("Failed to enable D3D12 debug layer");
                }

                dxgi_flags |= DXGI_CREATE_FACTORY_DEBUG;
            }

            unsafe { CreateDXGIFactory2(dxgi_flags) }.unwrap()
        };

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
                        std::ptr::null_mut(),
                        &mut cookie,
                    )
                }
                .unwrap();
            }
        }

        let queue = Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

        let rect_shader = RectShader::new(&device);

        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();

        let textures = SlotMap::new();

        let texture_descriptors = DescriptorHeap::new(
            &device,
            D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            1024,
            D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
        );

        let uploader = Uploader::new(&device, 1024 * 64);

        let frames = [
            Frame::new(&device, DEFAULT_DRAW_BUFFER_SIZE),
            Frame::new(&device, DEFAULT_DRAW_BUFFER_SIZE),
        ];

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &frames[0].cmda, None)
        }
        .unwrap();

        unsafe { command_list.Close() }.unwrap();

        let frames = VecDeque::from(frames);

        Self {
            dxgi,
            handle: device,
            queue,
            rect_shader,
            compositor,
            command_list,
            frames: RefCell::new(frames),
            uploader: RefCell::new(uploader),
            textures: RefCell::new(Box::new(textures)),
            texture_descriptors,
        }
    }

    pub fn idle(&self) {
        self.queue.wait_idle();
    }

    pub(super) fn get_texture(&self, id: TextureId) -> Ref<ID3D12Resource> {
        let lock = self.textures.borrow();
        let resource = Ref::map::<ID3D12Resource, _>(lock, |images| images.get(id).unwrap());
        resource
    }

    pub fn create_swapchain(&self, hwnd: HWND) -> Swapchain {
        Swapchain::new(self, hwnd)
    }

    pub fn create_texture(
        &self,
        extent: TextureExtent,
        layout: Layout,
        format: Format,
    ) -> TextureId {
        let format = to_dxgi_format(layout, format);

        let heap_desc = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 0,
            VisibleNodeMask: 0,
        };

        let image_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: extent.width as u64,
            Height: extent.height as u32,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };

        let mut image = None;

        unsafe {
            self.handle.CreateCommittedResource(
                &heap_desc,
                D3D12_HEAP_FLAG_NONE,
                &image_desc,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                None,
                &mut image,
            )
        }
        .unwrap();

        let id = self
            .textures
            .borrow_mut()
            .create(|key| {
                let image = image.unwrap();
                let view = self.texture_descriptors.cpu(key.index());

                unsafe { self.handle.CreateShaderResourceView(&image, None, view) };

                image
            })
            .unwrap();

        id
    }

    pub fn copy_raster_to_texture(
        &self,
        target: TextureId,
        pixels: &RasterBuf,
        origin: TexturePoint,
    ) {
        let target = self.get_texture(target);
        self.uploader
            .borrow_mut()
            .upload_image(&self.queue, &target, pixels, origin);
    }

    pub fn flush_upload_buffer(&self) {
        self.uploader.borrow_mut().flush_upload_buffer(&self.queue);
    }

    pub fn draw(&self, draw_list: &DrawList, target: &mut RenderTarget) {
        self.flush_upload_buffer();

        let mut frames = self.frames.borrow_mut();

        let mut frame = if let Some(frame) = frames.pop_front() {
            if self.queue.is_done(frame.sync) {
                frame
            } else {
                frames.push_front(frame);

                let size =
                    DEFAULT_DRAW_BUFFER_SIZE.max(std::mem::size_of_val(&draw_list.prims) as u64);

                Frame::new(&self.handle, size)
            }
        } else {
            panic!("no frames available")
        };

        frame.reset(&self.queue, &self.command_list);

        image_barrier(
            &self.command_list,
            &target.resource,
            target.state,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );

        frame.write(
            &self.handle,
            &self.command_list,
            draw_list,
            &self.rect_shader,
            target,
            &self.texture_descriptors,
        );

        image_barrier(
            &self.command_list,
            &target.resource,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            target.state,
        );

        unsafe { self.command_list.Close() }.unwrap();

        frame.sync = self.queue.submit(&self.command_list.cast().unwrap());
        target.draw = Some(frame.sync);

        frames.push_back(frame);
    }
}

/// A queue of GPU commands.
///
/// Based on the implementation described here: <https://alextardif.com/D3D11To12P1.html>
pub struct Queue {
    pub handle: ID3D12CommandQueue,
    fence: ID3D12Fence,
    num_submitted: AtomicU64,
    num_completed: AtomicU64,
}

impl Queue {
    fn new(device: &ID3D12Device, kind: D3D12_COMMAND_LIST_TYPE) -> Self {
        let queue = unsafe {
            device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type: kind,
                Priority: 0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            })
        }
        .unwrap();

        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        unsafe { fence.Signal(0) }.unwrap();

        Self {
            handle: queue,
            fence,
            num_submitted: AtomicU64::new(0),
            num_completed: AtomicU64::new(0),
        }
    }

    /// Causes the CPU to wait until the given submission has completed.
    ///
    /// # Returns
    ///
    /// `true` if the CPU had to wait, `false` if the submission has already completed.
    pub fn wait(&self, submission: SubmitId) -> bool {
        if self.is_done(submission) {
            return false;
        }

        unsafe {
            #[cfg(feature = "profile")]
            let _s = tracing_tracy::client::span!("wait for fence event");
            // absence of handle causes caller to block until fence reaches value
            self.fence
                .SetEventOnCompletion(submission.0, None)
                .expect("out of memory");
        }

        let _ = self
            .num_completed
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                (old < submission.0).then_some(submission.0)
            });

        true
    }

    /// Causes the CPU to wait until all submissions have completed.
    pub fn wait_idle(&self) {
        // We have to increment the fence value before waiting, because DXGI may
        // submit work to the queue on our behalf when we call `Present`.
        // Without this, we end up stomping over the currently presenting frame
        // when resizing or destroying the swapchain.
        let id = {
            // todo: relax ordering if possible
            let signal = self.num_submitted.fetch_add(1, Ordering::SeqCst);
            unsafe { self.handle.Signal(&self.fence, signal) }.unwrap();
            SubmitId(signal)
        };

        self.wait(id);
    }

    pub fn is_done(&self, submission: SubmitId) -> bool {
        if submission.0 > self.num_completed.load(Ordering::Acquire) {
            self.poll_fence();
        }

        submission.0 <= self.num_completed.load(Ordering::Acquire)
    }

    pub fn submit(&self, commands: &ID3D12CommandList) -> SubmitId {
        // todo: relax ordering if possible
        let signal = self.num_submitted.fetch_add(1, Ordering::SeqCst);

        unsafe { self.handle.ExecuteCommandLists(&[Some(commands.clone())]) };
        unsafe { self.handle.Signal(&self.fence, signal) }.unwrap();

        SubmitId(signal)
    }

    fn poll_fence(&self) {
        let fence_value = unsafe { self.fence.GetCompletedValue() };

        let _ = self
            .num_completed
            // Don't know what ordering to use here, so just use SeqCst for both
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                (old < fence_value).then_some(fence_value)
            });
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        self.wait_idle();
    }
}

unsafe extern "system" fn dx12_debug_callback(
    _category: D3D12_MESSAGE_CATEGORY,
    severity: D3D12_MESSAGE_SEVERITY,
    _id: D3D12_MESSAGE_ID,
    description: PCSTR,
    _context: *mut std::ffi::c_void,
) {
    #[allow(clippy::match_same_arms)]
    match severity {
        D3D12_MESSAGE_SEVERITY_CORRUPTION => eprintln!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_ERROR => eprintln!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_WARNING => eprintln!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_INFO => eprintln!("D3D12 {}", description.display()),
        D3D12_MESSAGE_SEVERITY_MESSAGE => eprintln!("D3D12 {}", description.display()),
        _ => {}
    }
}

pub struct DescriptorHeap {
    pub handle: ID3D12DescriptorHeap,
    capacity: u32,
    size: u32,
    cpu_base: usize,
    pub gpu_base: D3D12_GPU_DESCRIPTOR_HANDLE,
}

impl DescriptorHeap {
    pub fn new(
        device: &ID3D12Device,
        kind: D3D12_DESCRIPTOR_HEAP_TYPE,
        capacity: u32,
        flags: D3D12_DESCRIPTOR_HEAP_FLAGS,
    ) -> Self {
        let desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: kind,
            NumDescriptors: capacity,
            Flags: flags,
            NodeMask: 0,
        };

        let handle: ID3D12DescriptorHeap = unsafe { device.CreateDescriptorHeap(&desc) }.unwrap();

        let cpu_base = unsafe { handle.GetCPUDescriptorHandleForHeapStart().ptr };

        let gpu_base = unsafe { handle.GetGPUDescriptorHandleForHeapStart() };

        let size = unsafe { device.GetDescriptorHandleIncrementSize(kind) };

        Self {
            handle,
            capacity,
            size,
            cpu_base,
            gpu_base,
        }
    }

    pub fn cpu(&self, index: u32) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        assert!(index < self.capacity);

        let ptr = self.cpu_base + (index as usize * self.size as usize);
        D3D12_CPU_DESCRIPTOR_HANDLE { ptr }
    }
}

pub fn alloc_upload_buffer(device: &ID3D12Device, size: u64) -> ID3D12Resource {
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

pub struct Frame {
    buffer: Option<ID3D12Resource>,
    base: *mut u8,
    size: usize,
    sync: SubmitId,
    cmda: ID3D12CommandAllocator,
}

impl Frame {
    fn new(device: &ID3D12Device, size: u64) -> Self {
        let buffer = alloc_upload_buffer(device, size);

        let base = {
            let mut map = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut map)) }.unwrap();
            map.cast::<u8>()
        };

        let command_list_mem =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.unwrap();

        Self {
            buffer: Some(buffer),
            base,
            size: DEFAULT_DRAW_BUFFER_SIZE as usize,
            sync: SubmitId(0),
            cmda: command_list_mem,
        }
    }

    pub fn reset(&mut self, queue: &Queue, command_list: &ID3D12GraphicsCommandList) {
        queue.wait(self.sync);

        unsafe {
            self.cmda.Reset().unwrap();
            command_list.Reset(&self.cmda, None).unwrap();
        }
    }

    pub fn write(
        &mut self,
        device: &ID3D12Device,
        command_list: &ID3D12GraphicsCommandList,
        draw_list: &DrawList,
        shader: &RectShader,
        target: &RenderTarget,
        textures: &DescriptorHeap,
    ) {
        let content_size = std::mem::size_of_val(draw_list.prims.as_slice());

        if self.size < content_size {
            self.buffer = None;
            self.size = content_size;

            let buffer = alloc_upload_buffer(device, content_size as u64);

            self.base = {
                let mut map = std::ptr::null_mut();
                unsafe { buffer.Map(0, None, Some(&mut map)) }.unwrap();
                map.cast()
            };

            self.buffer = Some(buffer);
        }

        unsafe {
            self.base
                .copy_from_nonoverlapping(draw_list.prims.as_ptr().cast(), content_size);
        }

        let viewport_scale = [
            1.0 / f32::from(target.extent().width),
            1.0 / f32::from(target.extent().height),
        ];

        let mut it = draw_list.iter();

        assert_eq!(it.next(), Some(Command::Begin(draw_list.areas[0])));

        unsafe {
            command_list.OMSetRenderTargets(1, Some(&target.descriptor), false, None);

            command_list.RSSetViewports(&[D3D12_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: f32::from(target.extent().width),
                Height: f32::from(target.extent().height),
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]);

            command_list.RSSetScissorRects(&[RECT {
                left: 0,
                top: 0,
                right: i32::from(target.extent().width),
                bottom: i32::from(target.extent().height),
            }]);

            command_list.SetDescriptorHeaps(&[Some(textures.handle.clone())]);
        }

        shader.bind(
            command_list,
            self.buffer.as_ref().unwrap(),
            textures.gpu_base,
            viewport_scale,
            f32::from(target.extent().height),
        );

        let mut rect_start = 0;
        for command in it.by_ref() {
            match command {
                Command::Begin(_) => unreachable!(),
                Command::Close => break,
                Command::Clear(color) => unsafe {
                    command_list.ClearRenderTargetView(
                        target.descriptor,
                        &color.to_array_f32(),
                        None,
                    );
                },
                Command::Rects(count) => {
                    unsafe { command_list.DrawInstanced(4, count, 0, rect_start) };
                    rect_start += count;
                }
            }
        }
    }
}
