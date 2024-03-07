use std::marker::PhantomData;

use crate::{
    geometry::{DpiScale, Extent, Point, Wixel},
    graphics::{Canvas, FrameInfo, Graphics, GraphicsConfig, PixelBuf, WindowContext},
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
    pub images: &'static [(&'static str, PixelBuf<'static>)],
    pub graphics: GraphicsConfig,
}

pub struct Application {
    event_loop: EventLoop,
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
        // todo: make use of these
        #[allow(unused_variables)]
        let static_images = config.images;

        let event_loop = EventLoop::new()?;
        let graphics = Graphics::new(&config.graphics);

        Ok(Self {
            event_loop,
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
                outer: event_handler,
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
    event_loop: &'a ActiveEventLoop<(WindowState, UserWindowData)>,
}

impl<'a, UserWindowData> AppContext<'a, UserWindowData> {
    fn new(
        graphics: &'a Graphics,
        event_loop: &'a ActiveEventLoop<(WindowState, UserWindowData)>,
    ) -> Self {
        Self {
            graphics,
            event_loop,
        }
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
        &self,
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
    fn start(&mut self, app: &AppContext<WindowData>);

    fn suspend(&mut self, app: &AppContext<WindowData>) {}

    fn resume(&mut self, app: &AppContext<WindowData>) {}

    fn stop(&mut self);

    fn low_memory(&mut self, app: &AppContext<WindowData>) {}

    fn power_source_changed(&mut self, app: &AppContext<WindowData>, power_source: PowerSource) {}

    fn monitor_state_changed(&mut self, app: &AppContext<WindowData>, monitor: MonitorState) {}

    fn power_preference_changed(
        &mut self,
        app: &AppContext<WindowData>,
        power_preference: PowerPreference,
    ) {
    }

    fn activated(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn deactivated(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn drag_resize_started(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
    }

    fn drag_resize_ended(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {
    }

    fn resized(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        size: Extent<Wixel>,
    ) {
    }

    fn dpi_changed(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        dpi: DpiScale,
        size: Extent<Wixel>,
    ) {
    }

    #[allow(unused_variables)]
    fn close_requested(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {
        window.destroy();
    }

    fn shown(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn hidden(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn maximized(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn minimized(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn restored(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>) {}

    fn moved(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn wake_requested(&mut self, app: &AppContext<WindowData>, window: &mut Window<WindowData>);

    fn repaint(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        canvas: &mut Canvas,
        timing: &FrameInfo,
    );

    fn destroyed(&mut self, app: &AppContext<WindowData>, window_data: WindowData);

    fn key(
        // TODO: better name in the past tense
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    ) {
    }

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        button: MouseButton,
        state: ButtonState,
        position: Point<Wixel>,
        modifiers: ModifierKeys,
    ) {
    }

    fn mouse_scrolled(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
    }

    fn pointer_moved(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn pointer_entered(
        &mut self,
        app: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
        position: Point<Wixel>,
    ) {
    }

    fn pointer_left(
        &mut self,
        event_loop: &AppContext<WindowData>,
        window: &mut Window<WindowData>,
    ) {
    }
}

struct WindowState {
    context: WindowContext,
}

struct ApplicationEventHandler<'a, UserData, Outer: EventHandler<UserData>> {
    outer: Outer,
    graphics: &'a Graphics,
    phantom: PhantomData<UserData>,
}

impl<UserData, Outer: EventHandler<UserData>> SysEventHandler<(WindowState, UserData)>
    for ApplicationEventHandler<'_, UserData, Outer>
{
    fn start(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.start(&cx);
    }

    fn suspend(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.suspend(&cx);
    }

    fn resume(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.resume(&cx);
    }

    fn stop(&mut self) {
        self.outer.stop();
    }

    fn low_memory(&mut self, event_loop: &ActiveEventLoop<(WindowState, UserData)>) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.low_memory(&cx);
    }

    fn power_source_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        power_source: PowerSource,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.power_source_changed(&cx, power_source);
    }

    fn monitor_state_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        monitor: MonitorState,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.monitor_state_changed(&cx, monitor);
    }

    fn power_preference_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        power_preference: PowerPreference,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.power_preference_changed(&cx, power_preference);
    }

    fn activated(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.activated(&cx, &mut wn);
    }

    fn deactivated(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.deactivated(&cx, &mut wn);
    }

    fn drag_resize_started(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.drag_resize_started(&cx, &mut wn);
    }

    fn drag_resize_ended(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.drag_resize_ended(&cx, &mut wn);
    }

    fn resized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        size: Extent<Wixel>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.resize(size);
        self.outer.resized(&cx, &mut wn, size);
    }

    fn dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        dpi: DpiScale,
        size: Extent<Wixel>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.change_dpi(dpi, size);
        self.outer.dpi_changed(&cx, &mut wn, dpi, size);
    }

    fn close_requested(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.close_requested(&cx, &mut wn);
    }

    fn shown(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.shown(&cx, &mut wn);
    }

    fn hidden(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.hidden(&cx, &mut wn);
    }

    fn maximized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.maximized(&cx, &mut wn);
    }

    fn minimized(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.minimized(&cx, &mut wn);
    }

    fn restored(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.restored(&cx, &mut wn);
    }

    fn moved(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.moved(&cx, &mut wn, position);
    }

    fn wake_requested(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.wake_requested(&cx, &mut wn);
    }

    fn needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        _reason: PaintReason,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (meta, mut wn) = window.split();

        meta.context.draw(|canvas, frame| {
            self.outer.repaint(&cx, &mut wn, canvas, frame);
        });
    }

    fn destroyed(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        (_, window_data): (WindowState, UserData),
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        self.outer.destroyed(&cx, window_data);
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
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.key(&cx, &mut wn, code, state, modifiers);
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
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer
            .mouse_button(&cx, &mut wn, button, state, position, modifiers);
    }

    fn mouse_scrolled(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer
            .mouse_scrolled(&cx, &mut wn, delta, axis, modifiers);
    }

    fn pointer_moved(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.pointer_moved(&cx, &mut wn, position);
    }

    fn pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
        position: Point<Wixel>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.pointer_entered(&cx, &mut wn, position);
    }

    fn pointer_left(
        &mut self,
        event_loop: &ActiveEventLoop<(WindowState, UserData)>,
        window: Window<(WindowState, UserData)>,
    ) {
        let cx = AppContext::new(self.graphics, event_loop);
        let (_, mut wn) = window.split();
        self.outer.pointer_left(&cx, &mut wn);
    }
}
