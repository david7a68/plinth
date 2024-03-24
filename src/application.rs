use std::{collections::HashMap, marker::PhantomData};

use crate::{
    core::PassthroughBuildHasher,
    geometry::{Extent, Pixel, Point, Scale, Wixel},
    graphics::{Canvas, FrameInfo, Graphics, GraphicsConfig, Image, WindowContext},
    limits::{self, MAX_IMAGE_COUNT},
    resource::{Error as ResourceError, Resource, StaticResource},
    string::HashedStr,
    system::{
        event_loop::{ActiveEventLoop, EventHandler as SysEventHandler, EventLoop, EventLoopError},
        input::{ButtonState, KeyCode, ModifierKeys, MouseButton, ScrollAxis},
        power::{MonitorState, PowerPreference, PowerSource},
        window::{PaintReason, Window, WindowAttributes, WindowError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An error occurred in the event loop.")]
    EventLoop(#[from] EventLoopError),
}

#[derive(Debug, Default)]
pub struct Config {
    pub resources: &'static [StaticResource],
    pub graphics: GraphicsConfig,
}

pub struct Application {
    event_loop: EventLoop,
    resources: HashMap<u64, Resource, PassthroughBuildHasher>,
    graphics: Graphics,
}

impl Application {
    /// Initializes the application.
    ///
    /// Only one application may be initialized at a time.
    ///
    /// # Errors
    ///
    /// This will fail if the event loop could not be initialized, or if an
    /// application has already been initialized.
    pub fn new(config: Config) -> Result<Self, Error> {
        limits::MAX_IMAGE_COUNT.check(&config.resources.len());

        let mut graphics = Graphics::new(&config.graphics);

        let mut resources =
            HashMap::with_capacity_and_hasher(MAX_IMAGE_COUNT.get(), PassthroughBuildHasher::new());

        for resource in config.resources {
            match resource {
                StaticResource::Image(name, pixels) => {
                    let image = graphics.create_image(pixels.info()).unwrap();
                    graphics.upload_image(image, pixels).unwrap();
                    resources.insert(name.hash, Resource::Image(image));
                }
            }
        }

        graphics.flush_upload_buffer();

        let event_loop = EventLoop::new()?;

        Ok(Self {
            event_loop,
            resources,
            graphics,
        })
    }

    /// Runs the application is finished.
    ///
    /// This returns when all windows are closed. This may only be called once.
    ///
    /// # Errors
    ///
    /// This function returns an error if the event loop could not be initialized.
    pub fn run<WindowData, H: EventHandler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), Error> {
        self.event_loop
            .run(ApplicationEventHandler {
                client: event_handler,
                resources: &mut self.resources,
                graphics: &self.graphics,
                phantom: PhantomData,
            })
            .map_err(Error::EventLoop)
    }
}

/// A reference to the application context.
///
/// This is passed into event handler methods to allow the handler to interact
/// with the application.
pub struct AppContext<'a, UserWindowData> {
    graphics: &'a Graphics,
    resources: &'a mut HashMap<u64, Resource, PassthroughBuildHasher>,
    event_loop: &'a ActiveEventLoop<(WindowState<'a>, UserWindowData)>,
}

impl<'a, UserWindowData> AppContext<'a, UserWindowData> {
    fn new(
        graphics: &'a Graphics,
        resources: &'a mut HashMap<u64, Resource, PassthroughBuildHasher>,
        event_loop: &'a ActiveEventLoop<(WindowState, UserWindowData)>,
    ) -> Self {
        Self {
            graphics,
            resources,
            event_loop,
        }
    }

    /// Loads an image from a path.
    ///
    /// If the image is already loaded, this will return a reference to the
    /// existing image. Resources are cached to reduce the frequency of IO
    /// operations but may be evicted from the cache to make room for others.
    /// However, this will never happen while an event callback is running.
    /// Static images are guaranteed to be available for the lifetime of the
    /// app.
    ///
    /// # Errors
    ///
    ///  This function returns an error if the path is too long, or if the image
    ///  is missing or malformed. It may also return an IO error if one is
    ///  encountered.
    pub fn load_image(&mut self, path: HashedStr) -> Result<Image, ResourceError> {
        limits::MAX_RESOURCE_PATH_LENGTH.test(path.string, ResourceError::PathTooLong)?;
        match self.resources.entry(path.hash) {
            std::collections::hash_map::Entry::Occupied(entry) => match entry.get() {
                Resource::Image(image) => Ok(*image),
                #[allow(unreachable_patterns)]
                _ => Err(ResourceError::NotAnImage),
            },
            std::collections::hash_map::Entry::Vacant(_entry) => {
                // 3 parts: load header, create image, upload pixels
                todo!()
            }
        }
    }

    /// Loads a resource from a path.
    ///
    /// If the resource is already loaded, this will return a reference to the
    /// existing resource. Resources are cached to reduce the frequency of IO
    /// operations but may be evicted from the cache to make room for others.
    /// However, this will never happen while an event callback is running.
    /// Static resources are guaranteed to be available for the lifetime of the
    /// app.
    ///
    /// # Errors
    ///
    ///  This function returns an error if the path is too long, or if the
    ///  resource is missing or malformed. It may also return an IO error if one
    ///  is encountered.
    pub fn load_resource(&mut self, path: HashedStr) -> Result<Resource, ResourceError> {
        limits::MAX_RESOURCE_PATH_LENGTH.test(path.string, ResourceError::PathTooLong)?;
        let _ = path;
        todo!()
    }

    /// Creates a new window.
    ///
    /// # Errors
    ///
    /// This function returns an error if the window title exceeds
    /// [`MAX_WINDOW_TITLE_LENGTH`](crate::limits::MAX_WINDOW_TITLE_LENGTH)
    /// bytes, or if creating a new window would exceed the
    /// [`MAX_WINDOWS`](crate::limits::MAX_WINDOWS) limit.
    ///
    /// It may also return an error under platform-specific conditions.
    pub fn create_window(
        &mut self,
        attributes: WindowAttributes,
        constructor: impl FnOnce(Window<()>) -> UserWindowData,
    ) -> Result<(), WindowError> {
        self.event_loop.create_window(attributes, |window| {
            let context = self.graphics.create_window_context(window.hwnd());
            let user_data = constructor(window);
            (WindowState { context }, user_data)
        })
    }
}

#[allow(unused_variables)]
pub trait EventHandler<WindowData> {
    fn start(&mut self, app: &mut AppContext<WindowData>);

    fn suspend(&mut self, app: &mut AppContext<WindowData>) {}

    fn resume(&mut self, app: &mut AppContext<WindowData>) {}

    fn stop(&mut self);

    fn low_memory(&mut self, app: &mut AppContext<WindowData>) {}

    fn power_source_changed(
        &mut self,
        app: &mut AppContext<WindowData>,
        power_source: PowerSource,
    ) {
    }

    fn monitor_state_changed(&mut self, app: &mut AppContext<WindowData>, monitor: MonitorState) {}

    fn power_preference_changed(
        &mut self,
        app: &mut AppContext<WindowData>,
        power_preference: PowerPreference,
    ) {
    }

    fn activated(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn deactivated(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn drag_resize_started(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
    }

    fn drag_resize_ended(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
    }

    fn resized(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        size: Extent<Wixel>,
    ) {
    }

    fn dpi_changed(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        dpi: Scale<Wixel, Pixel>,
        size: Extent<Wixel>,
    ) {
    }

    #[allow(unused_variables)]
    fn close_requested(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
        window.destroy();
    }

    fn shown(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn hidden(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn maximized(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn minimized(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn restored(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn moved(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn wake_requested(&mut self, app: &mut AppContext<WindowData>, window: &mut Window<WindowData>);

    fn repaint(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        canvas: &mut Canvas,
        timing: &FrameInfo,
    );

    fn destroyed(&mut self, app: &mut AppContext<WindowData>, window_data: WindowData);

    fn key(
        // TODO: better name in the past tense
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    ) {
    }

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        button: MouseButton,
        state: ButtonState,
        position: Point<Wixel>,
        modifiers: ModifierKeys,
    ) {
    }

    fn mouse_scrolled(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
    }

    fn pointer_moved(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn pointer_entered(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn pointer_left(
        &mut self,
        event_loop: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
    }
}

struct WindowState<'a> {
    context: WindowContext<'a>,
}

struct ApplicationEventHandler<'a, UserData, Client: EventHandler<UserData>> {
    client: Client,
    graphics: &'a Graphics,
    resources: &'a mut HashMap<u64, Resource, PassthroughBuildHasher>,
    phantom: PhantomData<UserData>,
}

impl<UserData, Outer: EventHandler<UserData>> SysEventHandler<(WindowState<'_>, UserData)>
    for ApplicationEventHandler<'_, UserData, Outer>
{
    fn start(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.start(&mut cx);
    }

    fn suspend(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.suspend(&mut cx);
    }

    fn resume(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.resume(&mut cx);
    }

    fn stop(&mut self) {
        self.client.stop();
    }

    fn low_memory(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.low_memory(&mut cx);
    }

    fn power_source_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        power_source: PowerSource,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.power_source_changed(&mut cx, power_source);
    }

    fn monitor_state_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        monitor: MonitorState,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.monitor_state_changed(&mut cx, monitor);
    }

    fn power_preference_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        power_preference: PowerPreference,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client
            .power_preference_changed(&mut cx, power_preference);
    }

    fn activated(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.activated(&mut cx, &mut wn);
    }

    fn deactivated(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.deactivated(&mut cx, &mut wn);
    }

    fn drag_resize_started(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.drag_resize_started(&mut cx, &mut wn);
    }

    fn drag_resize_ended(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.drag_resize_ended(&mut cx, &mut wn);
    }

    fn resized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        size: Extent<Wixel>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.resize(size);
        self.client.resized(&mut cx, &mut wn, size);
    }

    fn dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        dpi: Scale<Wixel, Pixel>,
        size: Extent<Wixel>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.change_dpi(size, dpi);
        self.client.dpi_changed(&mut cx, &mut wn, dpi, size);
    }

    fn close_requested(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.close_requested(&mut cx, &mut wn);
    }

    fn shown(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.shown(&mut cx, &mut wn);
    }

    fn hidden(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.hidden(&mut cx, &mut wn);
    }

    fn maximized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.maximized(&mut cx, &mut wn);
    }

    fn minimized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.minimized(&mut cx, &mut wn);
    }

    fn restored(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.restored(&mut cx, &mut wn);
    }

    fn moved(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.moved(&mut cx, &mut wn, position);
    }

    fn wake_requested(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.wake_requested(&mut cx, &mut wn);
    }

    fn needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        _reason: PaintReason,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.draw(|canvas, frame| {
            self.client.repaint(&mut cx, &mut wn, canvas, frame);
        });
    }

    fn destroyed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        (_, window_data): (WindowState, UserData),
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        self.client.destroyed(&mut cx, window_data);
    }

    fn key(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.key(&mut cx, &mut wn, code, state, modifiers);
    }

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        button: MouseButton,
        state: ButtonState,
        position: Point<Wixel>,
        modifiers: ModifierKeys,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client
            .mouse_button(&mut cx, &mut wn, button, state, position, modifiers);
    }

    fn mouse_scrolled(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client
            .mouse_scrolled(&mut cx, &mut wn, delta, axis, modifiers);
    }

    fn pointer_moved(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.pointer_moved(&mut cx, &mut wn, position);
    }

    fn pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.pointer_entered(&mut cx, &mut wn, position);
    }

    fn pointer_left(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);
        let (_, mut wn) = window.split();
        self.client.pointer_left(&mut cx, &mut wn);
    }
}
