use std::rc::Rc;

use arrayvec::ArrayVec;
use euclid::Size2D;
use windows::{
    core::ComInterface,
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandAllocator,
                ID3D12CommandList, ID3D12CommandQueue, ID3D12Debug, ID3D12DescriptorHeap,
                ID3D12Device, ID3D12Fence, ID3D12Resource, D3D12_COMMAND_LIST_TYPE,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
                D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
                D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                D3D12_DESCRIPTOR_HEAP_TYPE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE,
                D3D12_GPU_DESCRIPTOR_HANDLE,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN,
                    DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, IDXGIFactory2, IDXGISwapChain2, DXGI_RGBA, DXGI_SCALING_NONE,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
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
    fn create_swapchain(&self, window: HWND) -> IDXGISwapChain2;
}

pub struct Dx12Renderer {
    dxgi_factory: IDXGIFactory2,
    device: ID3D12Device,

    graphics_queue: Queue,

    command_allocator: ID3D12CommandAllocator,

    render_target_descriptor_heap: SimpleDescriptorHeap<MAX_RENDER_TARGETS>,
}

impl Dx12Renderer {
    pub fn new(config: &GraphicsConfig) -> Self {
        if config.debug_mode {
            let mut controller: Option<ID3D12Debug> = None;
            unsafe { D3D12GetDebugInterface(&mut controller) }.unwrap();

            if let Some(controller) = controller {
                unsafe { controller.EnableDebugLayer() };
            }
        }

        let dxgi_flags = if config.debug_mode {
            windows::Win32::Graphics::Dxgi::DXGI_CREATE_FACTORY_DEBUG
        } else {
            0
        };

        let dxgi_factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(dxgi_flags) }.unwrap();

        let device = {
            let adapter = unsafe { dxgi_factory.EnumAdapters1(0) }.unwrap();

            let mut device: Option<ID3D12Device> = None;
            unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.unwrap();

            device.unwrap()
        };

        let graphics_queue = Queue::new(&device, D3D12_COMMAND_LIST_TYPE_DIRECT);

        let command_allocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.unwrap();

        let render_target_descriptor_heap =
            SimpleDescriptorHeap::new(&device, D3D12_DESCRIPTOR_HEAP_TYPE_RTV, false);

        Self {
            dxgi_factory,
            device,
            graphics_queue,
            command_allocator,
            render_target_descriptor_heap,
        }
    }
}

impl Renderer for Dx12Renderer {
    fn create_swapchain(&self, window: HWND) -> IDXGISwapChain2 {
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
        .cast::<IDXGISwapChain2>()
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

        swapchain
    }
}

/// Resize the swapchain to the given size.
///
/// If `size` is `None`, the swapchain will be resized to the size of the window.
pub fn resize_swapchain(swapchain: &IDXGISwapChain2, size: Option<Size2D<u16, ScreenSpace>>) {
    let size = size.unwrap_or_default().to_u32();

    unsafe {
        swapchain
            .ResizeBuffers(0, size.width, size.height, DXGI_FORMAT_UNKNOWN, 0)
            .unwrap();
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
        let heap: ID3D12DescriptorHeap = unsafe {
            device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: kind,
                NumDescriptors: COUNT as u32,
                Flags: shader_visible
                    .then_some(D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE)
                    .unwrap_or(D3D12_DESCRIPTOR_HEAP_FLAG_NONE),
                NodeMask: 0,
            })
        }
        .unwrap_or_else(|e| {
            tracing::error!("Failed to create descriptor heap: {:?}", e);
            panic!()
        });

        debug_assert!(COUNT <= u16::MAX as usize);

        Self {
            cpu_heap_start: unsafe { heap.GetCPUDescriptorHandleForHeapStart() },
            gpu_heap_start: shader_visible
                .then_some(unsafe { heap.GetGPUDescriptorHandleForHeapStart() })
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SubmissionId(u64);

/// A queue of GPU commands.
///
/// Based on the implementation described here: https://alextardif.com/D3D11To12P1.html
struct Queue {
    queue: ID3D12CommandQueue,
    fence: ID3D12Fence,
    fence_event: HANDLE,
    num_submitted: u64,
    num_completed: u64,
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
            num_submitted: 0,
            num_completed: 0,
        }
    }

    /// Causes the CPU to wait until the given submission has completed.
    pub fn wait(&mut self, submission: SubmissionId) {
        if self.is_done(submission) {
            return;
        }

        unsafe {
            self.fence
                .SetEventOnCompletion(submission.0, self.fence_event)
                .expect("out of memory");
            WaitForSingleObject(self.fence_event, INFINITE);
        }

        self.num_completed = submission.0;
    }

    /// Causes the CPU to wait until all submissions have completed.
    pub fn wait_idle(&mut self) {
        self.wait(SubmissionId(self.num_submitted - 1));
    }

    pub fn is_done(&mut self, submission: SubmissionId) -> bool {
        if submission.0 > self.num_completed {
            self.poll_fence();
        }

        submission.0 <= self.num_completed
    }

    pub fn submit(&mut self, commands: &ID3D12CommandList) -> SubmissionId {
        let id = self.num_submitted;

        unsafe {
            self.queue.ExecuteCommandLists(&[Some(commands.clone())]);
            self.queue.Signal(&self.fence, id).unwrap();
        }

        self.num_submitted += 1;
        SubmissionId(id)
    }

    fn poll_fence(&mut self) {
        self.num_completed = self
            .num_completed
            .max(unsafe { self.fence.GetCompletedValue() });
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
