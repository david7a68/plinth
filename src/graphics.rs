use std::rc::Rc;

use euclid::Size2D;
use windows::{
    core::ComInterface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_COMMAND_QUEUE_FLAG_NONE,
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
    },
};

use crate::window::ScreenSpace;

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
    graphics_queue: ID3D12CommandQueue,
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

        let graphics_queue = {
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
            graphics_queue,
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
            self.dxgi_factory
                .CreateSwapChainForHwnd(&self.graphics_queue, window, &swapchain_desc, None, None)
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to create swapchain: {:?}", e);
                    panic!()
                })
        };

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

        swapchain.cast().unwrap_or_else(|e| {
            tracing::error!(
                "The running version of windows doesn't support IDXGISwapchain2. Error: {:?}",
                e
            );
            panic!()
        })
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
