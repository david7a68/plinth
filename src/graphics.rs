use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use arrayvec::ArrayVec;
use euclid::Size2D;
use smallvec::SmallVec;
use windows::{
    core::{ComInterface, PCSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Graphics::{
            self,
            Direct3D::{
                Fxc::{D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION},
                D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_12_0,
            },
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandAllocator,
                ID3D12CommandList, ID3D12CommandQueue, ID3D12Debug, ID3D12Debug1, ID3D12Debug2,
                ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList,
                ID3D12InfoQueue1, ID3D12PipelineState, ID3D12Resource, D3D12_COMMAND_LIST_TYPE,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_CPU_DESCRIPTOR_HANDLE,
                D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE, D3D12_GPU_DESCRIPTOR_HANDLE,
                D3D12_MESSAGE_CALLBACK_FLAG_NONE, D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID,
                D3D12_MESSAGE_SEVERITY, D3D12_MESSAGE_SEVERITY_CORRUPTION,
                D3D12_MESSAGE_SEVERITY_ERROR, D3D12_MESSAGE_SEVERITY_INFO,
                D3D12_MESSAGE_SEVERITY_MESSAGE, D3D12_MESSAGE_SEVERITY_WARNING,
                D3D12_RENDER_TARGET_VIEW_DESC, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
                D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
                D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
                D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_RTV_DIMENSION_TEXTURE2D,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, IDXGIFactory2, IDXGISwapChain3, DXGI_CREATE_FACTORY_DEBUG,
                DXGI_PRESENT_RESTART, DXGI_RGBA, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::{CreateEventW, WaitForSingleObject, INFINITE},
    },
};

use crate::window::ScreenSpace;

pub const MAX_RENDER_TARGETS: usize = 32;

pub struct GraphicsConfig {
    pub debug_mode: bool,
}

impl GraphicsConfig {
    pub fn set_debug_mode(mut self, debug_mode: bool) -> Self {
        self.debug_mode = debug_mode;
        self
    }

    pub fn build(&self) -> Rc<dyn Renderer> {
        Rc::new(Dx12Renderer::new(self))
    }
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self { debug_mode: false }
    }
}

pub trait Renderer {
    fn create_swapchain(&self, window: HWND) -> Swapchain;

    /// Resizes the swapchain to the size of the window.
    fn resize_swapchain_auto(&self, swapchain: &Swapchain);

    /// Resizes the swapchain to the given size.
    ///
    /// Setting `flex_factor` causes the swapchain to adopt a slightly different
    /// mechanism for resizing in order to improve performance when resizing
    /// frequently (such as when the user is dragging a window border). If
    /// `size` is smaller than the swapchain's physical size, a sub-region of
    /// the swapchain is used for presentation. If `size` is larger than the
    /// swapchain's physical size, the swapchain is resized to `size *
    /// flex_factor` and the sub-region is adjusted to `size`. This allows us to
    /// avoid reallocating the swapchain's buffers on every frame, improving
    /// visual smoothness.
    fn resize_swapchain(
        &self,
        swapchain: &Swapchain,
        size: Size2D<u16, ScreenSpace>,
        flex_factor: Option<f32>,
    );

    fn present(&self, swapchain: &Swapchain);

    fn get_back_buffer<'a>(&self, swapchain: &'a Swapchain) -> (&'a Image, u32);

    fn new_graphics_context(&self) -> Box<dyn GraphicsContext>;

    fn submit_graphics_context(&self, context: Box<dyn GraphicsContext>) -> SubmissionId;

    fn wait(&self, until: SubmissionId);

    // fn wait_idle(&self)

    // fn create_buffer(&self, size, flags) -> Buffer

    // fn create_image(&self) -> Image

    // fn create_pipeline(&self, spec) -> Pipeline
}

pub trait GraphicsContext {
    /// Internal function, used to downcast to a concrete type.
    fn to_any(&mut self) -> &mut dyn Any;

    fn begin(&mut self);

    fn end(&mut self);

    fn set_render_target(&mut self, target: &Image);

    fn clear(&mut self, color: [f32; 4]);

    fn image_barrier(
        &mut self,
        image: &Image,
        from: D3D12_RESOURCE_STATES,
        to: D3D12_RESOURCE_STATES,
    );
}

pub struct Swapchain {
    handle: IDXGISwapChain3,
    images: [Image; 2],
}

pub struct Image {
    handle: ID3D12Resource,
    render_target_view: D3D12_CPU_DESCRIPTOR_HANDLE,
}

pub struct Dx12Renderer {
    dxgi_factory: IDXGIFactory2,
    device: ID3D12Device,

    graphics_queue: Queue,

    submissions: RefCell<VecDeque<ContextSubmission>>,

    graphics_contexts: RefCell<SmallVec<[Box<dyn GraphicsContext>; 4]>>,

    render_target_descriptor_heap: RefCell<SimpleDescriptorHeap<MAX_RENDER_TARGETS>>,
}

impl Dx12Renderer {
    pub fn new(config: &GraphicsConfig) -> Self {
        let mut dxgi_flags = 0;

        if config.debug_mode {
            let mut controller: Option<ID3D12Debug1> = None;
            unsafe { D3D12GetDebugInterface(&mut controller) }.unwrap();

            if let Some(controller) = controller {
                tracing::info!("Enabling D3D12 debug layer");
                unsafe { controller.EnableDebugLayer() };

                unsafe { controller.SetEnableGPUBasedValidation(true) };
            } else {
                tracing::warn!("Failed to enable D3D12 debug layer");
            }

            dxgi_flags |= DXGI_CREATE_FACTORY_DEBUG;
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

        let render_target_descriptor_heap = RefCell::new(SimpleDescriptorHeap::new(
            &device,
            D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            false,
        ));

        Self {
            dxgi_factory,
            device,
            graphics_queue,
            submissions: RefCell::new(VecDeque::with_capacity(1)),
            graphics_contexts: RefCell::default(),
            render_target_descriptor_heap,
        }
    }
}

impl Renderer for Dx12Renderer {
    fn create_swapchain(&self, window: HWND) -> Swapchain {
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
            self.dxgi_factory.CreateSwapChainForHwnd(
                &self.graphics_queue.queue,
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
                "The running version of windows doesn't support IDXGISwapchain2. Error: {:?}",
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

        let image0 = unsafe { swapchain.GetBuffer(0) }.unwrap();
        let view0 = self
            .render_target_descriptor_heap
            .borrow_mut()
            .allocate()
            .unwrap();

        let image1 = unsafe { swapchain.GetBuffer(1) }.unwrap();
        let view1 = self
            .render_target_descriptor_heap
            .borrow_mut()
            .allocate()
            .unwrap();

        unsafe {
            self.device.CreateRenderTargetView(&image0, None, view0);
            self.device.CreateRenderTargetView(&image1, None, view1);
        }

        Swapchain {
            handle: swapchain,
            images: [
                Image {
                    handle: image0,
                    render_target_view: view0,
                },
                Image {
                    handle: image1,
                    render_target_view: view1,
                },
            ],
        }
    }

    fn resize_swapchain_auto(&self, swapchain: &Swapchain) {
        unsafe { swapchain.handle.Present(0, DXGI_PRESENT_RESTART) }.unwrap();
        unsafe {
            swapchain
                .handle
                .ResizeBuffers(0, 0, 0, DXGI_FORMAT_UNKNOWN, 0)
        }
        .unwrap();
    }

    fn resize_swapchain(
        &self,
        swapchain: &Swapchain,
        size: Size2D<u16, ScreenSpace>,
        flex_factor: Option<f32>,
    ) {
        let source_size = size.cast::<u32>();

        unsafe { swapchain.handle.Present(0, DXGI_PRESENT_RESTART) }.unwrap();

        if let Some(flex_factor) = flex_factor {
            let mut desc = Default::default();
            unsafe { swapchain.handle.GetDesc1(&mut desc) }.unwrap();

            if source_size.width > desc.Width || source_size.height > desc.Height {
                let swapchain_size = (source_size.to_f32() * flex_factor)
                    .min(Size2D::splat(u16::MAX as f32))
                    .cast();

                resize_swapchain(&swapchain.handle, swapchain_size);
            }

            unsafe {
                swapchain
                    .handle
                    .SetSourceSize(source_size.width, source_size.height)
            }
            .unwrap();
        } else {
            resize_swapchain(&swapchain.handle, source_size);
        }
    }

    fn present(&self, swapchain: &Swapchain) {
        unsafe { swapchain.handle.Present(1, 0) }.unwrap();
    }

    fn get_back_buffer<'a>(&self, swapchain: &'a Swapchain) -> (&'a Image, u32) {
        let index = unsafe { swapchain.handle.GetCurrentBackBufferIndex() };
        let back_buffer = &swapchain.images[index as usize];
        (back_buffer, index)
    }

    fn new_graphics_context(&self) -> Box<dyn GraphicsContext> {
        let last_completed = self.graphics_queue.last_completed();
        let mut context_pool = self.graphics_contexts.borrow_mut();

        tracing::debug!(
            "Fetching graphics context. {} pending submissions",
            self.submissions.borrow().len()
        );

        while let Some(submission_id) = self.submissions.borrow().front().map(|s| s.submission_id) {
            if submission_id <= last_completed {
                let ContextSubmission {
                    submission_id: _,
                    context,
                } = unsafe { self.submissions.borrow_mut().pop_front().unwrap_unchecked() };

                context_pool.push(context);
            } else {
                break;
            }
        }

        let mut context = context_pool.pop().unwrap_or_else(|| {
            let context = Dx12GraphicsContext::new(&self.device);
            Box::new(context)
        });

        context.begin();
        context
    }

    fn submit_graphics_context(&self, mut context: Box<dyn GraphicsContext>) -> SubmissionId {
        let gc: &mut Dx12GraphicsContext = context.to_any().downcast_mut().unwrap();

        gc.end();

        let command_list = gc.command_list.cast().unwrap();
        let submission_id = self.graphics_queue.submit(&command_list);

        tracing::debug!(
            "Submitting graphics context. Submission id: {}",
            submission_id
        );

        let submission = ContextSubmission {
            submission_id,
            context,
        };

        self.submissions.borrow_mut().push_back(submission);

        submission_id
    }

    fn wait(&self, until: SubmissionId) {
        self.graphics_queue.wait(until);
    }
}

impl Drop for Dx12Renderer {
    fn drop(&mut self) {
        self.graphics_queue.wait_idle();
    }
}

struct Dx12GraphicsContext {
    command_list: ID3D12GraphicsCommandList,
    command_allocator: ID3D12CommandAllocator,

    render_target: Option<D3D12_CPU_DESCRIPTOR_HANDLE>,
}

impl Dx12GraphicsContext {
    pub fn new(device: &ID3D12Device) -> Self {
        let command_allocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
        }
        .unwrap();

        unsafe { command_list.Close() }.unwrap();

        Self {
            command_list,
            command_allocator,
            render_target: None,
        }
    }
}

impl GraphicsContext for Dx12GraphicsContext {
    fn to_any(&mut self) -> &mut dyn Any {
        self
    }

    fn begin(&mut self) {
        self.render_target = None;
        unsafe { self.command_allocator.Reset() }.unwrap();
        unsafe { self.command_list.Reset(&self.command_allocator, None) }.unwrap();
    }

    fn end(&mut self) {
        unsafe { self.command_list.Close() }.unwrap();
    }

    fn set_render_target(&mut self, target: &Image) {
        self.render_target = Some(target.render_target_view);

        unsafe {
            self.command_list
                .OMSetRenderTargets(1, Some(&target.render_target_view), false, None)
        };
    }

    fn image_barrier(
        &mut self,
        image: &Image,
        from: D3D12_RESOURCE_STATES,
        to: D3D12_RESOURCE_STATES,
    ) {
        let transition = D3D12_RESOURCE_TRANSITION_BARRIER {
            pResource: unsafe { std::mem::transmute_copy(&image.handle) },
            Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            StateBefore: from,
            StateAfter: to,
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

    fn clear(&mut self, color: [f32; 4]) {
        if let Some(render_target) = self.render_target {
            unsafe {
                self.command_list
                    .ClearRenderTargetView(render_target, color.as_ptr(), None)
            };
        }
    }
}

struct ContextSubmission {
    submission_id: SubmissionId,
    context: Box<dyn GraphicsContext>,
}

/// Resize the swapchain to the given size.
///
/// If `size` is `None`, the swapchain will be resized to the size of the window.
fn resize_swapchain(swapchain: &IDXGISwapChain3, size: Size2D<u32, ScreenSpace>) {
    unsafe { swapchain.ResizeBuffers(0, size.width, size.height, DXGI_FORMAT_UNKNOWN, 0) }.unwrap();
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

/// Uniquely identifies a submission to the GPU. Used to track when a submission
/// has completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubmissionId(u64);

impl std::fmt::Display for SubmissionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// A queue of GPU commands.
///
/// Based on the implementation described here: https://alextardif.com/D3D11To12P1.html
struct Queue {
    queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
    fence_event: HANDLE,
    num_submitted: Cell<u64>,
    num_completed: Cell<u64>,
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
            queue,
            fence,
            fence_event,
            num_submitted: Cell::new(0),
            num_completed: Cell::new(0),
        }
    }

    /// Causes the CPU to wait until the given submission has completed.
    pub fn wait(&self, submission: SubmissionId) {
        if self.is_done(submission) {
            return;
        }

        unsafe {
            self.fence
                .SetEventOnCompletion(submission.0, self.fence_event)
                .expect("out of memory");
            WaitForSingleObject(self.fence_event, INFINITE);
        }

        self.num_completed.set(submission.0);
    }

    /// Causes the CPU to wait until all submissions have completed.
    pub fn wait_idle(&self) {
        self.wait(SubmissionId(self.num_submitted.get() - 1));
    }

    pub fn is_done(&self, submission: SubmissionId) -> bool {
        if submission.0 > self.num_completed.get() {
            self.poll_fence();
        }

        submission.0 <= self.num_completed.get()
    }

    pub fn last_completed(&self) -> SubmissionId {
        self.poll_fence();
        SubmissionId(self.num_completed.get())
    }

    pub fn submit(&self, commands: &ID3D12CommandList) -> SubmissionId {
        let id = self.num_submitted.get();

        unsafe {
            self.queue.ExecuteCommandLists(&[Some(commands.clone())]);
            self.queue.Signal(&self.fence, id).unwrap();
        }

        self.num_submitted.set(id + 1);
        SubmissionId(id)
    }

    fn poll_fence(&self) {
        self.num_completed.set(
            self.num_completed
                .get()
                .max(unsafe { self.fence.GetCompletedValue() }),
        );
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        unsafe {
            self.wait_idle();
            CloseHandle(self.fence_event);
        }
    }
}

unsafe extern "system" fn dx12_debug_callback(
    category: D3D12_MESSAGE_CATEGORY,
    severity: D3D12_MESSAGE_SEVERITY,
    id: D3D12_MESSAGE_ID,
    description: PCSTR,
    context: *mut std::ffi::c_void,
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
