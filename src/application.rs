use std::{collections::HashMap, marker::PhantomData};

use crate::{
    core::{arena::Arena, PassthroughBuildHasher},
    graphics::{Canvas, DrawList, FrameInfo, Graphics, GraphicsConfig, Image, Swapchain},
    hash::HashedStr,
    limits::{ResourcePath, GFX_IMAGE_COUNT_MAX},
    resource::{Error as ResourceError, Resource, StaticResource},
    system::{
        event_loop::{
            ActiveEventLoop, AppEvent, Event, EventLoop, EventLoopError, Handler, WindowEvent,
        },
        ButtonState, DpiScale, KeyCode, ModifierKeys, MonitorState, MouseButton, PowerPreference,
        PowerSource, ScrollAxis, Window, WindowAttributes, WindowError, WindowExtent, WindowPoint,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An error occurred in the event loop.")]
    EventLoop(#[from] EventLoopError),
}

#[derive(Debug)]
pub struct Config {
    pub resources: &'static [StaticResource],
    pub graphics: GraphicsConfig,
    pub frame_arena_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resources: &[],
            graphics: GraphicsConfig::default(),
            frame_arena_size: 1024 * 1024, // 1 MiB
        }
    }
}

pub struct Application {
    event_loop: EventLoop,
    frame_arena: Arena,
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
    pub fn new(config: &Config) -> Result<Self, Error> {
        assert!(config.resources.len() <= GFX_IMAGE_COUNT_MAX);

        let frame_arena = Arena::new(config.frame_arena_size).unwrap();

        let mut graphics = Graphics::new(&config.graphics);

        let mut resources =
            HashMap::with_capacity_and_hasher(GFX_IMAGE_COUNT_MAX, PassthroughBuildHasher::new());

        // todo: use a thread pool a la rayon to load resources in parallel

        for resource in config.resources {
            match resource {
                StaticResource::Raster(name, pixels) => {
                    let image = graphics.create_raster_image(pixels.info()).unwrap();
                    graphics.upload_raster_image(image, pixels).unwrap();
                    resources.insert(name.hash.0, Resource::Image(image));
                }
            }
        }

        graphics.flush_upload_buffer();

        let event_loop = EventLoop::new()?;

        Ok(Self {
            event_loop,
            frame_arena,
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
                frame_arena: &mut self.frame_arena,
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
    event_loop: Option<&'a ActiveEventLoop<(WindowState<'a>, Option<UserWindowData>)>>,
}

impl<'a, UserWindowData> AppContext<'a, UserWindowData> {
    fn new(
        graphics: &'a Graphics,
        resources: &'a mut HashMap<u64, Resource, PassthroughBuildHasher>,
        event_loop: &'a ActiveEventLoop<(WindowState, Option<UserWindowData>)>,
    ) -> Self {
        Self {
            graphics,
            resources,
            event_loop: Some(event_loop),
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
        let _path_str = ResourcePath::new(path).ok_or(ResourceError::PathTooLong)?;

        match self.resources.entry(path.hash.0) {
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
        let _path_str = ResourcePath::new(path).ok_or(ResourceError::PathTooLong)?;
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
        self.event_loop
            .ok_or(WindowError::ExitingEventLoop)?
            .create_window(attributes, |window| {
                let swapchain = self.graphics.create_swapchain(window.hwnd());
                let user_data = constructor(window);

                (
                    WindowState {
                        swapchain,
                        draw_list: DrawList::new(),
                        dpi_scale: DpiScale::IDENTITY,
                        to_resize: WindowExtent::ZERO,
                    },
                    Some(user_data),
                )
            })
    }
}

#[allow(unused_variables)]
pub trait EventHandler<WindowData> {
    fn start(&mut self, app: &mut AppContext<WindowData>);

    fn suspend(&mut self, app: &mut AppContext<WindowData>) {}

    fn resume(&mut self, app: &mut AppContext<WindowData>) {}

    fn stop(&mut self, app: &mut AppContext<WindowData>);

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
        size: WindowExtent,
    ) {
    }

    fn dpi_changed(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        dpi: DpiScale,
        size: WindowExtent,
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
        position: WindowPoint,
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
        position: WindowPoint,
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
        position: WindowPoint,
    ) {
    }

    fn pointer_entered(
        &mut self,
        app: &mut AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: WindowPoint,
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
    swapchain: Swapchain<'a>,
    draw_list: DrawList,
    dpi_scale: DpiScale,
    to_resize: WindowExtent,
}

struct ApplicationEventHandler<'a, UserData, Client: EventHandler<UserData>> {
    client: Client,
    frame_arena: &'a mut Arena,
    resources: &'a mut HashMap<u64, Resource, PassthroughBuildHasher>,
    graphics: &'a Graphics,
    phantom: PhantomData<UserData>,
}

impl<'a, UserData, Client: EventHandler<UserData>> Handler<(WindowState<'a>, Option<UserData>)>
    for ApplicationEventHandler<'a, UserData, Client>
{
    fn handle(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, Option<UserData>)>,
        event: Event<(WindowState, Option<UserData>)>,
    ) {
        let mut cx = AppContext::new(self.graphics, self.resources, event_loop);

        match event {
            Event::App(event) => match event {
                AppEvent::Start => self.client.start(&mut cx),
                AppEvent::Suspend => self.client.suspend(&mut cx),
                AppEvent::Resume => self.client.resume(&mut cx),
                AppEvent::Stop => self.client.stop(&mut cx),
                AppEvent::LowMemory => self.client.low_memory(&mut cx),
                AppEvent::PowerSource(source) => self.client.power_source_changed(&mut cx, source),
                AppEvent::MonitorState(monitor) => {
                    self.client.monitor_state_changed(&mut cx, monitor)
                }
                AppEvent::PowerPreference(preference) => {
                    self.client.power_preference_changed(&mut cx, preference)
                }
            },
            Event::Window(window, event) => {
                let (mut meta, mut win) = window.split();

                match event {
                    WindowEvent::Activate => self
                        .client
                        .activated(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Deactivate => self
                        .client
                        .deactivated(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::DragResize(start) => {
                        if start {
                            self.client
                                .drag_resize_started(&mut cx, &mut win.extract_option().unwrap())
                        } else {
                            self.client
                                .drag_resize_ended(&mut cx, &mut win.extract_option().unwrap())
                        }
                    }
                    WindowEvent::Resize(size) => {
                        meta.to_resize = size;
                        self.client
                            .resized(&mut cx, &mut win.extract_option().unwrap(), size)
                    }
                    WindowEvent::DpiChange(dpi, size) => {
                        meta.dpi_scale = dpi;
                        meta.to_resize = size;
                        self.client.dpi_changed(
                            &mut cx,
                            &mut win.extract_option().unwrap(),
                            dpi,
                            size,
                        )
                    }
                    WindowEvent::CloseRequest => self
                        .client
                        .close_requested(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Shown => self
                        .client
                        .shown(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Hidden => self
                        .client
                        .hidden(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Maximized => self
                        .client
                        .maximized(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Minimized => self
                        .client
                        .minimized(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Restored => self
                        .client
                        .restored(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Move(new_pos) => {
                        self.client
                            .moved(&mut cx, &mut win.extract_option().unwrap(), new_pos)
                    }
                    WindowEvent::Wake => self
                        .client
                        .wake_requested(&mut cx, &mut win.extract_option().unwrap()),
                    WindowEvent::Repaint(_) => {
                        if meta.to_resize != WindowExtent::ZERO {
                            meta.swapchain.resize(std::mem::take(&mut meta.to_resize));
                            meta.to_resize = WindowExtent::ZERO;
                        }

                        let mut image = meta.swapchain.next_image();

                        meta.draw_list.clear();

                        // hacky
                        let scale = DpiScale::new(meta.dpi_scale.factor);

                        self.frame_arena.reset();

                        let mut canvas = self.graphics.create_canvas(
                            self.frame_arena,
                            &image,
                            &mut meta.draw_list,
                            scale,
                        );

                        self.client.repaint(
                            &mut cx,
                            &mut win.extract_option().unwrap(),
                            &mut canvas,
                            &image.frame_info(),
                        );

                        // in case the client didn't call finish
                        canvas.finish();

                        self.graphics.draw(&meta.draw_list, &mut image);

                        image.present();
                    }
                    WindowEvent::Destroy => {
                        // todo: this is a hack
                        unsafe { std::ptr::drop_in_place(&mut meta) };

                        self.client
                            .destroyed(&mut cx, win.data_mut().take().unwrap())
                    }
                    WindowEvent::Key(code, state, modifiers) => self.client.key(
                        &mut cx,
                        &mut win.extract_option().unwrap(),
                        code,
                        state,
                        modifiers,
                    ),
                    WindowEvent::MouseButton(button, state, position, modifiers) => {
                        self.client.mouse_button(
                            &mut cx,
                            &mut win.extract_option().unwrap(),
                            button,
                            state,
                            position,
                            modifiers,
                        )
                    }
                    WindowEvent::MouseScrolled(delta, axis, modifiers) => {
                        self.client.mouse_scrolled(
                            &mut cx,
                            &mut win.extract_option().unwrap(),
                            delta,
                            axis,
                            modifiers,
                        )
                    }
                    WindowEvent::PointerMoved(position) => self.client.pointer_moved(
                        &mut cx,
                        &mut win.extract_option().unwrap(),
                        position,
                    ),
                    WindowEvent::PointerEntered(position) => self.client.pointer_entered(
                        &mut cx,
                        &mut win.extract_option().unwrap(),
                        position,
                    ),
                    WindowEvent::PointerLeft => self
                        .client
                        .pointer_left(&mut cx, &mut win.extract_option().unwrap()),
                }
            }
        }
    }
}
