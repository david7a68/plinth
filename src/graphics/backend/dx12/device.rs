use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{
    lock_api::{MappedMutexGuard, MutexGuard},
    Mutex, RawMutex,
};
use windows::{
    core::{Interface, PCSTR},
    Win32::Graphics::{
        Direct3D::D3D_FEATURE_LEVEL_12_0,
        Direct3D12::{
            D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandList, ID3D12CommandQueue,
            ID3D12Debug1, ID3D12Debug5, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
            ID3D12GraphicsCommandList, ID3D12InfoQueue1, ID3D12Resource, D3D12_COMMAND_LIST_TYPE,
            D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
            D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_CPU_DESCRIPTOR_HANDLE,
            D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAGS,
            D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE,
            D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, D3D12_FENCE_FLAG_NONE,
            D3D12_GPU_DESCRIPTOR_HANDLE, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
            D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN,
            D3D12_MESSAGE_CALLBACK_FLAG_NONE, D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID,
            D3D12_MESSAGE_SEVERITY, D3D12_MESSAGE_SEVERITY_CORRUPTION,
            D3D12_MESSAGE_SEVERITY_ERROR, D3D12_MESSAGE_SEVERITY_INFO,
            D3D12_MESSAGE_SEVERITY_MESSAGE, D3D12_MESSAGE_SEVERITY_WARNING, D3D12_RESOURCE_DESC,
            D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_GENERIC_READ,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_SHADER_RESOURCE_VIEW_DESC,
            D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_TEX2D_SRV,
            D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_TEXTURE_LAYOUT_UNKNOWN,
        },
        DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
        Dxgi::{
            Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
            CreateDXGIFactory2, IDXGIFactory2, IDXGISwapChain1, DXGI_CREATE_FACTORY_DEBUG,
            DXGI_SWAP_CHAIN_DESC1,
        },
    },
};

use crate::{
    core::static_slot_map::SlotMap,
    geometry::{Extent, Texel},
    graphics::{
        backend::{SubmitId, TextureId},
        Format, GraphicsConfig, Layout,
    },
};

use super::{shaders::RectShader, to_dxgi_format};

pub struct Device_ {
    dxgi: IDXGIFactory2,
    pub handle: ID3D12Device,
    queue: Queue,
    pub rect_shader: RectShader,
    pub compositor: IDCompositionDevice,

    textures: Mutex<SlotMap<1024, ID3D12Resource, TextureId>>,
    pub texture_descriptors: DescriptorHeap,
}

impl Device_ {
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

        let textures = Mutex::new(SlotMap::new());

        let texture_descriptors = DescriptorHeap::new(
            &device,
            D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            1024,
            D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
        );

        Self {
            dxgi,
            handle: device,
            queue,
            rect_shader,
            compositor,
            textures,
            texture_descriptors,
        }
    }

    pub fn create_swapchain(&self, desc: &DXGI_SWAP_CHAIN_DESC1) -> IDXGISwapChain1 {
        unsafe {
            self.dxgi
                .CreateSwapChainForComposition(&self.queue.handle, desc, None)
        }
        .unwrap_or_else(|e| {
            eprintln!("Failed to create swapchain: {:?}", e);
            panic!();
        })
    }

    /// Causes the CPU to wait until the given submission has completed.
    ///
    /// # Returns
    ///
    /// `true` if the CPU had to wait, `false` if the submission has already completed.
    pub fn wait(&self, submit_id: SubmitId) -> bool {
        self.queue.wait(submit_id)
    }

    pub fn idle(&self) {
        self.queue.wait_idle();
    }

    pub fn submit(&self, command_list: &ID3D12GraphicsCommandList) -> SubmitId {
        self.queue.submit(&command_list.cast().unwrap())
    }

    pub fn create_buffer(&self, size: u64) -> ID3D12Resource {
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
            self.handle.CreateCommittedResource(
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

    pub(super) fn get_texture(&self, id: TextureId) -> MappedMutexGuard<RawMutex, ID3D12Resource> {
        let lock = self.textures.lock();

        let resource: MappedMutexGuard<RawMutex, ID3D12Resource> =
            MutexGuard::map::<ID3D12Resource, _>(lock, |images| images.get_mut(id).unwrap());

        resource
    }

    pub fn create_texture(
        &self,
        extent: Extent<Texel>,
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
            Width: extent.width.0 as u64,
            Height: extent.height.0 as u32,
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
            .lock()
            .create(|key| {
                let image = image.unwrap();
                let descriptor = self.texture_descriptors.cpu(key.index());

                unsafe {
                    self.handle
                        .CreateShaderResourceView(&image, None, descriptor)
                };

                image
            })
            .unwrap();

        id
    }
}

/// A queue of GPU commands.
///
/// Based on the implementation described here: <https://alextardif.com/D3D11To12P1.html>
struct Queue {
    handle: ID3D12CommandQueue,
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
    fn wait(&self, submission: SubmitId) -> bool {
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
    fn wait_idle(&self) {
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

    fn is_done(&self, submission: SubmitId) -> bool {
        if submission.0 > self.num_completed.load(Ordering::Acquire) {
            self.poll_fence();
        }

        submission.0 <= self.num_completed.load(Ordering::Acquire)
    }

    fn submit(&self, commands: &ID3D12CommandList) -> SubmitId {
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
