use crate::{
    graphics::{Graphics, GraphicsConfig},
    system::{
        dpi::{DpiScale, WindowPoint, WindowSize},
        input::{ButtonState, KeyCode, ModifierKeys, MouseButton, ScrollAxis},
        power::{MonitorState, PowerPreference, PowerSource},
        window::{PaintReason, Window, WindowAttributes, WindowError},
        ActiveEventLoop, EventHandler as SysEventHandler, EventLoop, EventLoopError,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An error occurred in the event loop.")]
    EventLoop(#[from] EventLoopError),
}

pub struct Application {
    event_loop: EventLoop,
    graphics: Graphics,
}

impl Application {
    pub fn new(graphics: &GraphicsConfig) -> Result<Self, Error> {
        let event_loop = EventLoop::new()?;
        let graphics = Graphics::new(graphics);

        Ok(Self {
            event_loop,
            graphics,
        })
    }

    /// Runs the application is finished.
    ///
    /// This returns when all windows are closed. This may only be called once.
    pub fn run<WindowData, H: SysEventHandler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), Error> {
        self.event_loop.run(event_handler).map_err(Error::EventLoop)
    }
}

pub struct AppContext<'a, UserWindowData> {
    event_loop: &'a ActiveEventLoop<WindowState<UserWindowData>>,
}

impl<'a, UserWindowData> AppContext<'a, UserWindowData> {
    fn new(event_loop: &'a ActiveEventLoop<WindowState<UserWindowData>>) -> Self {
        Self { event_loop }
    }

    pub fn create_window(
        &self,
        attributes: WindowAttributes,
        constructor: impl FnOnce(&Window<()>) -> UserWindowData + 'static,
    ) -> Result<(), WindowError> {
        self.event_loop.create_window(attributes, |window| {
            let user_data = constructor(window);
            WindowState { user_data }
        })
    }
}

pub trait EventHandler<UserWindowData> {
    fn start(&mut self, app: &AppContext<UserWindowData>);

    fn suspend(&mut self, app: &AppContext<UserWindowData>);

    fn resume(&mut self, app: &AppContext<UserWindowData>);

    fn stop(&mut self);

    fn low_memory(&mut self, app: &AppContext<UserWindowData>);

    fn power_source_changed(&mut self, app: &AppContext<UserWindowData>, power_source: PowerSource);

    fn monitor_state_changed(&mut self, app: &AppContext<UserWindowData>, monitor: MonitorState);

    fn power_preference_changed(
        &mut self,
        app: &AppContext<UserWindowData>,
        power_preference: PowerPreference,
    );

    fn activated(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn deactivated(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
    );

    fn drag_resize_started(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
    );

    fn drag_resize_ended(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
    );

    fn resized(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        size: WindowSize,
    );

    fn dpi_changed(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        dpi: DpiScale,
        size: WindowSize,
    );

    fn close_requested(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
    ) {
        window.destroy();
    }

    fn shown(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn hidden(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn maximized(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn minimized(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn restored(&mut self, app: &AppContext<UserWindowData>, window: &mut Window<UserWindowData>);

    fn moved(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        position: WindowPoint,
    );

    fn wake_requested(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
    );

    fn needs_repaint(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        reason: PaintReason,
    );

    fn destroyed(&mut self, app: &AppContext<UserWindowData>, window_data: UserWindowData);

    fn key(
        // TODO: better name in the past tense
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    );

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        button: MouseButton,
        state: ButtonState,
        position: WindowPoint,
        modifiers: ModifierKeys,
    );

    fn mouse_scrolled(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    );

    fn pointer_moved(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        position: WindowPoint,
    );

    fn pointer_entered(
        &mut self,
        app: &AppContext<UserWindowData>,
        window: &mut Window<UserWindowData>,
        position: WindowPoint,
    );

    fn pointer_left(
        &mut self,
        event_loop: &ActiveEventLoop<UserWindowData>,
        window: &mut Window<UserWindowData>,
    );
}

struct WindowState<UserData> {
    user_data: UserData,
}

struct ApplicationEventHandler<UserData, Outer: EventHandler<UserData>> {
    outer: Outer,
    graphics: Graphics,
    phantom: std::marker::PhantomData<UserData>,
}

impl<UserData, Outer: EventHandler<UserData>> SysEventHandler<WindowState<UserData>>
    for ApplicationEventHandler<UserData, Outer>
{
    fn start(&mut self, event_loop: &ActiveEventLoop<WindowState<UserData>>) {
        let cx = AppContext::new(event_loop);
        self.outer.start(&cx);
    }

    fn suspend(&mut self, event_loop: &ActiveEventLoop<WindowState<UserData>>) {
        let cx = AppContext::new(event_loop);
        self.outer.suspend(&cx);
    }

    fn resume(&mut self, event_loop: &ActiveEventLoop<WindowState<UserData>>) {
        let cx = AppContext::new(event_loop);
        self.outer.resume(&cx);
    }

    fn stop(&mut self) {
        self.outer.stop();
    }

    fn low_memory(&mut self, event_loop: &ActiveEventLoop<WindowState<UserData>>) {
        let cx = AppContext::new(event_loop);
        self.outer.low_memory(&cx);
    }

    fn power_source_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        power_source: PowerSource,
    ) {
        let cx = AppContext::new(event_loop);
        self.outer.power_source_changed(&cx, power_source);
    }

    fn monitor_state_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        monitor: MonitorState,
    ) {
        let cx = AppContext::new(event_loop);
        self.outer.monitor_state_changed(&cx, monitor);
    }

    fn power_preference_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        power_preference: PowerPreference,
    ) {
        let cx = AppContext::new(event_loop);
        self.outer.power_preference_changed(&cx, power_preference);
    }

    fn activated(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        let cx = AppContext::new(event_loop);
        let mut wn = window.map(|d| &mut d.user_data);
        self.outer.activated(&cx, &mut wn);
    }

    fn deactivated(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        let cx = AppContext::new(event_loop);
        let mut wn = window.map(|d| &mut d.user_data);
        self.outer.deactivated(&cx, &mut wn);
    }

    fn drag_resize_started(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn drag_resize_ended(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn resized(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        size: crate::system::dpi::WindowSize,
    ) {
        todo!()
    }

    fn dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        dpi: crate::system::dpi::DpiScale,
        size: crate::system::dpi::WindowSize,
    ) {
        todo!()
    }

    fn close_requested(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn shown(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn hidden(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn maximized(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn minimized(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn restored(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn moved(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        position: crate::system::dpi::WindowPoint,
    ) {
        todo!()
    }

    fn wake_requested(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }

    fn needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        reason: crate::system::window::PaintReason,
    ) {
        todo!()
    }

    fn destroyed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window_data: WindowState<UserData>,
    ) {
        todo!()
    }

    fn key(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        code: crate::system::input::KeyCode,
        state: crate::system::input::ButtonState,
        modifiers: crate::system::input::ModifierKeys,
    ) {
        todo!()
    }

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        button: crate::system::input::MouseButton,
        state: crate::system::input::ButtonState,
        position: crate::system::dpi::WindowPoint,
        modifiers: crate::system::input::ModifierKeys,
    ) {
        todo!()
    }

    fn mouse_scrolled(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        delta: f32,
        axis: crate::system::input::ScrollAxis,
        modifiers: crate::system::input::ModifierKeys,
    ) {
        todo!()
    }

    fn pointer_moved(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        position: crate::system::dpi::WindowPoint,
    ) {
        todo!()
    }

    fn pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
        position: crate::system::dpi::WindowPoint,
    ) {
        todo!()
    }

    fn pointer_left(
        &mut self,
        event_loop: &ActiveEventLoop<WindowState<UserData>>,
        window: Window<WindowState<UserData>>,
    ) {
        todo!()
    }
}
