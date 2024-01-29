use std::{
    borrow::Cow,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use parking_lot::RwLock;
use slotmap::SlotMap;
use windows::Win32::Graphics::DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice};

use crate::{
    graphics::{GraphicsConfig, Image},
    io::{self, LocationId},
    platform::dx12,
    window::{WindowError, WindowSpec},
    EventHandler, Window,
};

use super::{
    loader::{spawn_resource_thread, LoaderMessage},
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

    pub fn add_resource_location(&mut self, location: impl io::Location) -> io::LocationId {
        self.context.add_resource_location(location)
    }

    pub fn add_image_loader(
        &mut self,
        location: io::LocationId,
        loader: impl io::ImageLoader + Send + 'static,
    ) -> Result<(), io::Error> {
        self.context.add_image_loader(location, loader)
    }

    pub fn load_image(
        &mut self,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.context.load_image(path)
    }

    pub fn load_image_from_location(
        &mut self,
        location: io::LocationId,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.context.load_image_from_location(location, path)
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
    // Don't put senders in Arc since they're refcounted anyway. Not sure if
    // this is the way to go. -dz
    pub inner: Arc<Win32Context>,
    pub sender: Sender<AppMessage>,
    pub vsync_sender: Sender<VSyncRequest>,
    pub fs_location: LocationId,
}

impl AppContextImpl {
    #[tracing::instrument(skip(config))]
    fn new(
        config: &GraphicsConfig,
        sender: Sender<AppMessage>,
        vsync_sender: Sender<VSyncRequest>,
    ) -> Self {
        let inner = {
            let dx12 = dx12::Device::new(config);
            let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();
            let locations = SlotMap::with_capacity_and_key(1);

            Arc::new(Win32Context {
                dx12,
                compositor,
                locations: RwLock::new(locations),
            })
        };

        let fs_location = {
            let send = spawn_resource_thread(inner.clone(), io::fs::FileSystem::new());
            inner.locations.write().insert(send)
        };

        #[cfg(any(feature = "png", feature = "jpeg"))]
        {
            let loader = io::image::DefaultLoader::new();
            inner
                .locations
                .read()
                .get(fs_location)
                .unwrap()
                .send(LoaderMessage::AddImageLoader(Box::new(loader)))
                .unwrap();
        }

        Self {
            inner,
            sender,
            vsync_sender,
            fs_location,
        }
    }

    pub fn add_resource_location(&mut self, location: impl io::Location) -> io::LocationId {
        let send = spawn_resource_thread(self.inner.clone(), location);
        self.inner.locations.write().insert(send)
    }

    pub fn add_image_loader(
        &mut self,
        location: io::LocationId,
        loader: impl io::ImageLoader + Send + 'static,
    ) -> Result<(), io::Error> {
        let locations = self.inner.locations.read();

        let location = locations
            .get(location)
            .ok_or_else(|| io::Error::InvalidLocation(location))?;

        location
            .send(LoaderMessage::AddImageLoader(Box::new(loader)))
            .unwrap();

        Ok(())
    }

    pub fn load_image(
        &mut self,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        self.load_image_from_location(self.fs_location, path)
    }

    pub fn load_image_from_location(
        &mut self,
        location: io::LocationId,
        path: Cow<'static, str>,
    ) -> io::AsyncLoad<Result<Image, io::Error>> {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let message = LoaderMessage::LoadImage(path, send);
        self.inner.locations.read()[location].send(message).unwrap();

        io::AsyncLoad::new(recv)
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
    pub locations: RwLock<SlotMap<io::LocationId, Sender<LoaderMessage>>>,
}

unsafe impl Send for Win32Context {}
unsafe impl Sync for Win32Context {}
