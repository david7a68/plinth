use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use parking_lot::RwLock;
use windows::{
    core::ComInterface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D12::{D3D12GetDebugInterface, ID3D12Debug1, ID3D12Debug5},
            DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
            Dxgi::{CreateDXGIFactory2, IDXGIFactory2, DXGI_CREATE_FACTORY_DEBUG},
        },
    },
};

use crate::{
    graphics::GraphicsConfig,
    platform::{
        dx12,
        win32::{event_loop::spawn_event_loop, ui_thread::spawn_ui_thread},
    },
    window::{WindowError, WindowSpec},
    Window, WindowEventHandler,
};

use super::{
    swapchain::Swapchain,
    vsync::{VsyncRequest, VsyncThread},
};

pub enum AppMessage {
    WindowCreate,
    WindowDestroy,
}

pub struct ApplicationImpl {
    context: AppContextImpl,
    app_receiver: Receiver<AppMessage>,
    vsync_request_receiver: Receiver<VsyncRequest>,
}

impl ApplicationImpl {
    #[tracing::instrument(skip(graphics))]
    pub fn new(graphics: &GraphicsConfig) -> Self {
        let (app_sender, app_receiver) = std::sync::mpsc::channel();

        let (vsync_request_sender, vsync_request_receiver) = std::sync::mpsc::channel();

        let context = AppContextImpl::new(graphics, app_sender, vsync_request_sender);

        Self {
            context,
            app_receiver,
            vsync_request_receiver,
        }
    }

    pub fn spawn_window<W, F>(
        &mut self,
        spec: WindowSpec,
        constructor: F,
    ) -> Result<(), WindowError>
    where
        W: WindowEventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.context.spawn_window(spec, constructor)
    }

    pub fn run(&mut self) {
        let mut window_count = 0;
        let mut vsync = VsyncThread::new(&self.context, &self.vsync_request_receiver);

        // block on messages to start
        match self.app_receiver.recv() {
            Ok(AppMessage::WindowCreate) => window_count += 1,
            Ok(AppMessage::WindowDestroy) => window_count -= 1,
            Err(_) => return,
        }

        loop {
            while let Ok(msg) = self.app_receiver.try_recv() {
                match msg {
                    AppMessage::WindowCreate => window_count += 1,
                    AppMessage::WindowDestroy => window_count -= 1,
                }
            }

            if window_count <= 0 {
                break;
            }

            vsync.tick();
        }
    }
}

pub struct Win32Context {
    pub dxgi: IDXGIFactory2,
    pub dx12: Arc<dx12::Device>,
    pub compositor: IDCompositionDevice,
    debug_mode: bool,
}

impl Win32Context {
    fn new(config: &GraphicsConfig) -> Self {
        let dxgi: IDXGIFactory2 = create_factory(config.debug_mode);
        let dx12 = dx12::Device::new(&dxgi, config);

        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();

        Self {
            dxgi,
            dx12,
            compositor,
            debug_mode: config.debug_mode,
        }
    }

    pub fn update_device(&mut self) {
        self.dxgi = create_factory(self.debug_mode);
    }
}

unsafe impl Send for Win32Context {}
unsafe impl Sync for Win32Context {}

#[derive(Clone)]
pub struct AppContextImpl {
    pub inner: Arc<RwLock<Win32Context>>,
    pub sender: Sender<AppMessage>,
    pub vsync_sender: Sender<VsyncRequest>,
}

impl AppContextImpl {
    #[tracing::instrument(skip(config))]
    fn new(
        config: &GraphicsConfig,
        sender: Sender<AppMessage>,
        vsync_sender: Sender<VsyncRequest>,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Win32Context::new(config))),
            sender,
            vsync_sender,
        }
    }

    pub fn spawn_window<W, F>(
        &mut self,
        spec: WindowSpec,
        constructor: F,
    ) -> Result<(), WindowError>
    where
        W: WindowEventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
        let (ui_sender, ui_receiver) = std::sync::mpsc::channel();

        spawn_event_loop(spec, self.sender.clone(), ui_sender);
        spawn_ui_thread(self.clone(), constructor, ui_receiver);

        Ok(())
    }

    pub fn create_swapchain(&self, hwnd: HWND) -> Swapchain {
        let this = self.inner.read();
        Swapchain::new(&this.dxgi, &this.compositor, this.dx12.queue(), hwnd)
    }
}

fn create_factory(debug: bool) -> IDXGIFactory2 {
    let mut dxgi_flags = 0;

    if debug {
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
    }

    unsafe { CreateDXGIFactory2(dxgi_flags) }.unwrap()
}
