use parking_lot::Mutex;
use windows::{
    core::{ComInterface, PCSTR},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandList, ID3D12Debug1,
                ID3D12Debug5, ID3D12Device, ID3D12InfoQueue1, D3D12_COMMAND_LIST_TYPE_DIRECT,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_MESSAGE_CALLBACK_FLAG_NONE,
                D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_ID, D3D12_MESSAGE_SEVERITY,
                D3D12_MESSAGE_SEVERITY_CORRUPTION, D3D12_MESSAGE_SEVERITY_ERROR,
                D3D12_MESSAGE_SEVERITY_INFO, D3D12_MESSAGE_SEVERITY_MESSAGE,
                D3D12_MESSAGE_SEVERITY_WARNING,
            },
            Dxgi::{
                CreateDXGIFactory2, DXGIGetDebugInterface1, IDXGIDebug, IDXGIFactory2,
                DXGI_CREATE_FACTORY_DEBUG, DXGI_DEBUG_ALL, DXGI_DEBUG_RLO_ALL,
                DXGI_DEBUG_RLO_IGNORE_INTERNAL,
            },
        },
    },
};

use crate::{application::GraphicsConfig, graphics::SubmissionId};

use super::{Dx12GraphicsCommandList, Dx12Swapchain, Queue, SimpleDescriptorHeap};

pub const MAX_RENDER_TARGETS: usize = 32;

pub struct Dx12Device {
    pub dxgi_factory: IDXGIFactory2,
    pub device: ID3D12Device,

    pub graphics_queue: Queue,
    pub render_target_descriptor_heap: Mutex<SimpleDescriptorHeap<MAX_RENDER_TARGETS>>,
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

    pub fn create_swapchain(&self, window: HWND) -> Dx12Swapchain {
        Dx12Swapchain::new(self, window)
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_for_idle(&self) {
        self.graphics_queue.wait_idle();
    }

    #[tracing::instrument(skip(self))]
    pub fn wait_for_submission(&self, submission_id: SubmissionId) {
        self.graphics_queue.wait(submission_id);
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
