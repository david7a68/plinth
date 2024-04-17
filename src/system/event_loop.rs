use super::{
    input::{ButtonState, KeyCode, ModifierKeys, MouseButton, ScrollAxis},
    platform_impl,
    power::{MonitorState, PowerPreference, PowerSource},
    window::{PaintReason, Window, WindowAttributes, WindowError},
    {DpiScale, WindowExtent, WindowPoint},
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
pub enum EventLoopError {
    #[error("The operating system version is not supported.")]
    UnsupportedOsVersion,

    #[error("A platform error occurred.")]
    Platform(#[from] platform_impl::EventLoopError),
}

#[allow(unused_variables)]
pub trait EventHandler<WindowData> {
    fn start(&mut self, event_loop: &ActiveEventLoop<WindowData>);

    fn suspend(&mut self, event_loop: &ActiveEventLoop<WindowData>);

    fn resume(&mut self, event_loop: &ActiveEventLoop<WindowData>);

    fn stop(&mut self);

    fn low_memory(&mut self, event_loop: &ActiveEventLoop<WindowData>);

    fn power_source_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        power_source: PowerSource,
    );

    fn monitor_state_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        monitor: MonitorState,
    );

    fn power_preference_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        power_preference: PowerPreference,
    );

    fn activated(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn deactivated(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn drag_resize_started(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
    );

    fn drag_resize_ended(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
    );

    fn resized(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        size: WindowExtent,
    );

    fn dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        dpi: DpiScale,
        size: WindowExtent,
    );

    fn close_requested(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
    );

    fn shown(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn hidden(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn maximized(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn minimized(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn restored(&mut self, event_loop: &ActiveEventLoop<WindowData>, window: Window<WindowData>);

    fn moved(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        position: WindowPoint,
    );

    fn wake_requested(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
    );

    fn needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        reason: PaintReason,
    );

    fn destroyed(&mut self, event_loop: &ActiveEventLoop<WindowData>, window_data: WindowData);

    fn key(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    );

    fn mouse_button(
        // TODO: better name in the past tense
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        button: MouseButton,
        state: ButtonState,
        position: WindowPoint,
        modifiers: ModifierKeys,
    );

    fn mouse_scrolled(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    );

    fn pointer_moved(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        position: WindowPoint,
    );

    fn pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
        position: WindowPoint,
    );

    fn pointer_left(
        &mut self,
        event_loop: &ActiveEventLoop<WindowData>,
        window: Window<WindowData>,
    );
}

/// An event loop for the platform's windowing system.
///
/// Every window created by this event loop must use the same type of event
/// handler.
pub struct ActiveEventLoop<WindowData> {
    pub(crate) event_loop: platform_impl::ActiveEventLoop<WindowData>,
}

impl<WindowData> ActiveEventLoop<WindowData> {
    /// Creates a new window and constructs the event handler upon it.
    pub fn create_window(
        &self,
        attributes: WindowAttributes,
        constructor: impl FnOnce(Window<()>) -> WindowData,
    ) -> Result<(), WindowError> {
        self.event_loop.create_window(attributes, constructor)
    }
}

pub struct EventLoop {
    pub(crate) event_loop: platform_impl::EventLoop,
}

impl EventLoop {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Result<Self, EventLoopError> {
        Ok(Self {
            event_loop: platform_impl::EventLoop::new()?,
        })
    }

    pub fn run<WindowData, H: EventHandler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), EventLoopError> {
        self.event_loop.run(event_handler)
    }
}
