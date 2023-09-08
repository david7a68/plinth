use std::{
    cell::Cell,
    sync::atomic::{AtomicU64, Ordering},
};

use arrayvec::ArrayVec;
use euclid::Size2D;
use parking_lot::Mutex;
use windows::{
    core::{ComInterface, PCSTR},
    w,
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandAllocator,
                ID3D12CommandList, ID3D12CommandQueue, ID3D12Debug1, ID3D12Debug5,
                ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList,
                ID3D12InfoQueue1, ID3D12Resource, D3D12_COMMAND_LIST_TYPE,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE, D3D12_GPU_DESCRIPTOR_HANDLE,
                D3D12_MESSAGE_CALLBACK_FLAG_NONE, D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID,
                D3D12_MESSAGE_SEVERITY, D3D12_MESSAGE_SEVERITY_CORRUPTION,
                D3D12_MESSAGE_SEVERITY_ERROR, D3D12_MESSAGE_SEVERITY_INFO,
                D3D12_MESSAGE_SEVERITY_MESSAGE, D3D12_MESSAGE_SEVERITY_WARNING,
                D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
                D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
                D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_TRANSITION_BARRIER,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, DXGIGetDebugInterface1, IDXGIDebug, IDXGIFactory2,
                IDXGISwapChain3, DXGI_CREATE_FACTORY_DEBUG, DXGI_DEBUG_ALL, DXGI_DEBUG_RLO_ALL,
                DXGI_DEBUG_RLO_IGNORE_INTERNAL, DXGI_PRESENT_DO_NOT_SEQUENCE, DXGI_PRESENT_RESTART,
                DXGI_RGBA, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::{CreateEventW, WaitForSingleObject, INFINITE},
    },
};

use crate::shell::ScreenSpace;

use super::{GraphicsConfig, Image, ResizeOp, ResourceState, SubmissionId};

pub const MAX_RENDER_TARGETS: usize = 32;

pub struct Dx12Swapchain {
    handle: IDXGISwapChain3,
    images: Option<[Image; 2]>,
    was_resized: bool,
    last_present: Option<SubmissionId>,
}

impl Dx12Swapchain {
    pub fn new(device: &Dx12Device, window: HWND) -> Self {
        let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: 0,  // extract from hwnd
            Height: 0, // extract from hwnd
            Format: DXGI_FORMAT_R16G16B16A16_FLOAT,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,   // required by FLIP_SEQUENTIAL
                Quality: 0, // required by FLIP_SEQUENTIAL
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE, // backbuffer tranparency is ignored
            Flags: 0,
        };

        let swapchain = unsafe {
            device.dxgi_factory.CreateSwapChainForHwnd(
                &device.graphics_queue.queue.lock().0,
                window,
                &swapchain_desc,
                None,
                None,
            )
        }
        .unwrap_or_else(|e| {
            tracing::error!("Failed to create swapchain: {:?}", e);
            panic!()
        })
        .cast::<IDXGISwapChain3>()
        .unwrap_or_else(|e| {
            tracing::error!(
                "The running version of windows doesn't support IDXGISwapchain3. Error: {:?}",
                e
            );
            panic!()
        });

        unsafe {
            swapchain
                .SetBackgroundColor(&DXGI_RGBA {
                    r: 0.0,
                    g: 0.2,
                    b: 0.4,
                    a: 1.0,
                })
                .unwrap();
        }

        let images = Self::get_images(&swapchain, device);
        Self {
            handle: swapchain,
            images: Some(images),
            was_resized: false,
            last_present: None,
        }
    }

    pub fn resize(&mut self, device: &Dx12Device, op: ResizeOp) {
        fn resize_swapchain(
            device: &Dx12Device,
            swapchain: &mut Dx12Swapchain,
            size: Size2D<u32, ScreenSpace>,
        ) {
            if let Some(last_present) = swapchain.last_present {
                device.wait_until(last_present);
            }

            {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("free swapchain images");

                let images = swapchain.images.take().unwrap();
                let image0: &Dx12Image = (&images[0]).try_into().unwrap();
                let image1: &Dx12Image = (&images[1]).try_into().unwrap();

                {
                    let mut rt = device.render_target_descriptor_heap.lock();
                    rt.deallocate(image0.render_target_view);
                    rt.deallocate(image1.render_target_view);
                }

                std::mem::drop(images);
            }

            unsafe {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("resize buffers");

                swapchain
                    .handle
                    .ResizeBuffers(0, size.width, size.height, DXGI_FORMAT_UNKNOWN, 0)
            }
            .unwrap();

            swapchain.images = Some(Dx12Swapchain::get_images(&swapchain.handle, device));
        }

        match op {
            ResizeOp::Auto => resize_swapchain(device, self, Size2D::zero()),
            ResizeOp::Fixed { size } => resize_swapchain(device, self, size.cast()),
            ResizeOp::Flex { size, flex } => {
                let mut desc = Default::default();
                unsafe { self.handle.GetDesc1(&mut desc) }.unwrap();

                let size = size.cast::<u32>();
                if size.width > desc.Width || size.height > desc.Height {
                    let swapchain_size = (size.to_f32() * flex)
                        .min(Size2D::splat(u16::MAX as f32))
                        .cast();

                    resize_swapchain(device, self, swapchain_size);
                }

                unsafe { self.handle.SetSourceSize(size.width, size.height) }.unwrap();
            }
        }

        self.was_resized = true;
    }

    pub fn get_back_buffer(&self) -> (&Image, u32) {
        let index = unsafe { self.handle.GetCurrentBackBufferIndex() };
        let image = &(self.images.as_ref().unwrap())[index as usize];
        (image, index)
    }

    pub fn present(&mut self, submission_id: SubmissionId) {
        let flags = if self.was_resized {
            self.was_resized = false;
            DXGI_PRESENT_RESTART
        } else {
            0
        };

        unsafe { self.handle.Present(1, flags) }.unwrap();
        self.last_present = Some(submission_id);
    }

    #[tracing::instrument(skip(swapchain, device))]
    fn get_images(swapchain: &IDXGISwapChain3, device: &Dx12Device) -> [Image; 2] {
        let image0: ID3D12Resource = unsafe { swapchain.GetBuffer(0) }.unwrap();
        let image1: ID3D12Resource = unsafe { swapchain.GetBuffer(1) }.unwrap();

        let (view0, view1) = {
            let mut rt = device.render_target_descriptor_heap.lock();
            (rt.allocate().unwrap(), rt.allocate().unwrap())
        };

        unsafe {
            image0.SetName(w!("Backbuffer 0")).unwrap();
            image1.SetName(w!("Backbuffer 1")).unwrap();

            device.device.CreateRenderTargetView(&image0, None, view0);
            device.device.CreateRenderTargetView(&image1, None, view1);
        }

        [
            Dx12Image {
                handle: image0,
                render_target_view: view0,
            }
            .into(),
            Dx12Image {
                handle: image1,
                render_target_view: view1,
            }
            .into(),
        ]
    }
}

unsafe impl Send for Dx12Swapchain {}

pub struct Dx12Image {
    handle: ID3D12Resource,
    render_target_view: D3D12_CPU_DESCRIPTOR_HANDLE,
}

impl<'a> TryFrom<&'a Image> for &'a Dx12Image {
    type Error = ();

    fn try_from(wrapper: &'a Image) -> Result<Self, Self::Error> {
        match &wrapper.image {
            super::ImageImpl::Dx12(image) => Ok(image),
        }
    }
}

pub struct Dx12Device {
    dxgi_factory: IDXGIFactory2,
    device: ID3D12Device,

    graphics_queue: Queue,
    render_target_descriptor_heap: Mutex<SimpleDescriptorHeap<MAX_RENDER_TARGETS>>,
}

impl Dx12Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        let mut dxgi_flags = 0;

        if config.debug_mode {
            let mut controller: Option<ID3D12Debug1> = None;
            unsafe { D3D12GetDebugInterface(&mut controller) }.unwrap();

            if let Some(controller) = controller {
                tracing::info!("Enabling D3D12 debug layer");
                unsafe { controller.EnableDebugLayer() };

                unsafe { controller.SetEnableGPUBasedValidation(true) };

                if let Ok(controller) = controller.cast::<ID3D12Debug5>() {
                    unsafe { controller.SetEnableAutoName(true) };
                }
            } else {
                tracing::warn!("Failed to enable D3D12 debug layer");
            }

            dxgi_flags |= DXGI_CREATE_FACTORY_DEBUG;

            if let Ok(dxgi_debug) = unsafe { DXGIGetDebugInterface1::<IDXGIDebug>(0) } {
                tracing::info!("Enabling DXGI debug layer");

                unsafe {
                    dxgi_debug.ReportLiveObjects(
                        DXGI_DEBUG_ALL,
                        DXGI_DEBUG_RLO_ALL | DXGI_DEBUG_RLO_IGNORE_INTERNAL,
                    )
                }
                .unwrap();
            }
        }

        let dxgi_factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(dxgi_flags) }.unwrap();

        let device = {
            let adapter = unsafe { dxgi_factory.EnumAdapters1(0) }.unwrap();

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

        let graphics_queue = Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

        let render_target_descriptor_heap = Mutex::new(SimpleDescriptorHeap::new(
            &device,
            D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            false,
        ));

        Self {
            dxgi_factory,
            device,
            graphics_queue,
            render_target_descriptor_heap,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_for_idle(&self) {
        self.graphics_queue.wait_idle();
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_until(&self, submission: SubmissionId) {
        self.graphics_queue.wait(submission);
    }

    pub fn most_recently_completed_submission(&self) -> SubmissionId {
        self.graphics_queue.last_completed()
    }

    pub fn submit_graphics_command_list(&self, cmd_list: &Dx12GraphicsCommandList) -> SubmissionId {
        let cmd_list = cmd_list.command_list.cast::<ID3D12CommandList>().unwrap();
        self.graphics_queue.submit(&cmd_list)
    }
}

unsafe impl Send for Dx12Device {}
unsafe impl Sync for Dx12Device {}

pub struct Dx12GraphicsCommandList {
    command_list: ID3D12GraphicsCommandList,
    command_allocator: ID3D12CommandAllocator,

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

        unsafe { command_list.SetName(w!("Graphics Command List")) }.unwrap();
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
                .ClearRenderTargetView(render_target, color.as_ptr(), None)
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

struct SimpleDescriptorHeap<const COUNT: usize> {
    cpu_heap_start: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_heap_start: D3D12_GPU_DESCRIPTOR_HANDLE,
    handle_size: u32,
    indices: ArrayVec<u16, COUNT>,
    heap: ID3D12DescriptorHeap,
}

impl<const COUNT: usize> SimpleDescriptorHeap<COUNT> {
    pub fn new(
        device: &ID3D12Device,
        kind: D3D12_DESCRIPTOR_HEAP_TYPE,
        shader_visible: bool,
    ) -> Self {
        let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: kind,
            NumDescriptors: COUNT as u32,
            Flags: shader_visible
                .then_some(D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)
                .unwrap_or(D3D12_DESCRIPTOR_HEAP_FLAG_NONE),
            NodeMask: 0,
        };

        let heap: ID3D12DescriptorHeap = unsafe { device.CreateDescriptorHeap(&heap_desc) }
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create descriptor heap: {:?}", e);
                panic!()
            });

        debug_assert!(COUNT <= u16::MAX as usize);

        Self {
            cpu_heap_start: unsafe { heap.GetCPUDescriptorHandleForHeapStart() },
            gpu_heap_start: shader_visible
                .then(|| unsafe { heap.GetGPUDescriptorHandleForHeapStart() })
                .unwrap_or_default(),
            handle_size: unsafe { device.GetDescriptorHandleIncrementSize(kind) },
            indices: (0..COUNT).map(|i| i as u16).collect(),
            heap,
        }
    }

    pub fn gpu_handle(
        &self,
        cpu_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    ) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        D3D12_GPU_DESCRIPTOR_HANDLE {
            ptr: self.gpu_heap_start.ptr + self.check_cpu_offset(cpu_handle) as u64,
        }
    }

    pub fn allocate(&mut self) -> Option<D3D12_CPU_DESCRIPTOR_HANDLE> {
        self.indices.pop().map(|i| D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: self.cpu_heap_start.ptr + usize::try_from(i as u32 * self.handle_size).unwrap(),
        })
    }

    pub fn deallocate(&mut self, handle: D3D12_CPU_DESCRIPTOR_HANDLE) {
        let index = self.check_cpu_offset(handle) / self.handle_size as usize;
        debug_assert!(!self.indices.contains(&(index as u16)));
        self.indices.push(index as u16);
    }

    fn check_cpu_offset(&self, handle: D3D12_CPU_DESCRIPTOR_HANDLE) -> usize {
        let offset = handle.ptr - self.cpu_heap_start.ptr;
        debug_assert!((0..self.handle_size as usize * COUNT).contains(&offset));
        debug_assert!(offset % self.handle_size as usize == 0);
        offset
    }
}

/// A queue of GPU commands.
///
/// Based on the implementation described here: <https://alextardif.com/D3D11To12P1.html>
struct Queue {
    queue: Mutex<(ID3D12CommandQueue, u64)>,
    fence: ID3D12Fence,
    fence_event: Mutex<HANDLE>,
    num_completed: AtomicU64,
}

impl Queue {
    pub fn new(device: &ID3D12Device, kind: D3D12_COMMAND_LIST_TYPE) -> Self {
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
        let fence_event = unsafe { CreateEventW(None, false, false, None) }.unwrap();

        unsafe { fence.Signal(0) }.unwrap();

        Self {
            queue: Mutex::new((queue, 0)),
            fence,
            fence_event: Mutex::new(fence_event),
            num_completed: AtomicU64::new(0),
        }
    }

    /// Causes the CPU to wait until the given submission has completed.
    #[tracing::instrument(skip(self))]
    pub fn wait(&self, submission: SubmissionId) {
        if self.is_done(submission) {
            return;
        }

        {
            // TODO: this would be faster if we could use an event per thread.

            let event = {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("wait for lock");

                self.fence_event.lock()
            };

            unsafe {
                self.fence
                    .SetEventOnCompletion(submission.0, *event)
                    .expect("out of memory");
            }

            unsafe {
                #[cfg(feature = "profile")]
                let _s = tracing_tracy::client::span!("wait for fence event");

                WaitForSingleObject(*event, INFINITE);
            }
        }

        let _ = self
            .num_completed
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                (old < submission.0).then_some(submission.0)
            });
    }

    /// Causes the CPU to wait until all submissions have completed.
    #[tracing::instrument(skip(self))]
    pub fn wait_idle(&self) {
        // We have to increment the fence value before waiting, because DXGI may
        // submit work to the queue on our behalf when we call `Present`.
        // Without this, we end up stomping over the currently presenting frame
        // when resizing or destroying the swapchain.
        let id = {
            let (queue, num_submitted) = &mut *self.queue.lock();
            unsafe { queue.Signal(&self.fence, *num_submitted) }.unwrap();
            let id = SubmissionId(*num_submitted);
            *num_submitted += 1;
            id
        };

        self.wait(id);
    }

    pub fn is_done(&self, submission: SubmissionId) -> bool {
        if submission.0 > self.num_completed.load(Ordering::Acquire) {
            self.poll_fence();
        }

        submission.0 <= self.num_completed.load(Ordering::Acquire)
    }

    pub fn last_completed(&self) -> SubmissionId {
        self.poll_fence();
        SubmissionId(self.num_completed.load(Ordering::Acquire))
    }

    #[tracing::instrument(skip(self))]
    pub fn submit(&self, commands: &ID3D12CommandList) -> SubmissionId {
        let (queue, num_submitted) = &mut *self.queue.lock();

        let id = SubmissionId(*num_submitted);
        unsafe { queue.ExecuteCommandLists(&[Some(commands.clone())]) };
        unsafe { queue.Signal(&self.fence, *num_submitted) }.unwrap();
        *num_submitted += 1;

        id
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

        let event = self.fence_event.lock();
        unsafe { CloseHandle(*event) };
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
