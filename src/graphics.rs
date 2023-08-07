use std::rc::Rc;

use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct3D::D3D_FEATURE_LEVEL_11_0,
        Direct3D12::{
            D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device, D3D12_COMMAND_LIST_TYPE_DIRECT,
            D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
        },
        Dxgi::{CreateDXGIFactory2, IDXGIFactory2, IDXGISwapChain2},
    },
};

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
    fn create_swapchain(&self, window: HWND) -> IDXGISwapChain2 {
        todo!()
    }
}

pub struct Dx12Renderer {
    dxgi_factory: IDXGIFactory2,
    device: ID3D12Device,
    queue: ID3D12CommandQueue,
}

impl Dx12Renderer {
    pub fn new(config: &GraphicsConfig) -> Self {
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

        let queue = {
            let desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                Priority: 0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            };

            unsafe { device.CreateCommandQueue(&desc) }.unwrap()
        };

        Self {
            dxgi_factory,
            device,
            queue,
        }
    }
}

impl Renderer for Dx12Renderer {}
