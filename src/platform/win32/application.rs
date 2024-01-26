use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use windows::Win32::Graphics::DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice};

use crate::{
    graphics::GraphicsConfig,
    platform::dx12,
    window::{WindowError, WindowSpec},
    EventHandler, Window,
};

use super::{
    vsync::{VSyncRequest, VsyncThread},
    window::spawn_window_thread,
};

pub enum AppMessage {
    WindowCreate,
    WindowDestroy,
}

pub struct ApplicationImpl {
    context: AppContextImpl,
    app_receiver: Receiver<AppMessage>,
    vsync_request_receiver: Receiver<VSyncRequest>,
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
        W: EventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
        self.context.spawn_window(spec, constructor)
    }

    pub fn run(&mut self) {
        let mut window_count = 0;
        let mut vsync = VsyncThread::new(&self.vsync_request_receiver);

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

#[derive(Clone)]
pub struct AppContextImpl {
    pub inner: Arc<Win32Context>,
    pub sender: Sender<AppMessage>,
    pub vsync_sender: Sender<VSyncRequest>,
}

impl AppContextImpl {
    #[tracing::instrument(skip(config))]
    fn new(
        config: &GraphicsConfig,
        sender: Sender<AppMessage>,
        vsync_sender: Sender<VSyncRequest>,
    ) -> Self {
        Self {
            inner: Arc::new(Win32Context::new(config)),
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
        W: EventHandler,
        F: FnMut(Window) -> W + Send + 'static,
    {
        // todo: select interposer based on active graphics api - dz
        spawn_window_thread(self.clone(), spec, constructor, dx12::Interposer::new);

        // todo: error handling -dz
        Ok(())
    }
}

pub struct Win32Context {
    pub dx12: Arc<dx12::Device>,
    pub compositor: IDCompositionDevice,
}

impl Win32Context {
    fn new(config: &GraphicsConfig) -> Self {
        let dx12 = dx12::Device::new(config);

        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();

        Self { dx12, compositor }
    }
}

unsafe impl Send for Win32Context {}
unsafe impl Sync for Win32Context {}
