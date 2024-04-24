//! Abstraction over platform event loop.
//!
//! ## Implementation Details
//!
//! The event handler trait was used as a callback interface for the event loop.
//! However, this had a few issues: an adapter trait was needed to bridge the
//! event handler with the application event handler which added significant
//! boilerplate, and the trait made it difficult to serialize events for replay.
//!
//! The current approach is to use a callback closure for the event handler that
//! switches on a monolithic event enum. This makes it simple to serialize a
//! stream of events, and eliminates the event handler trait entirely.
//!
//! This is exactly the opposite of the approach taken by the `winit` crate,
//! which transitioned from a monolithic event enum to a trait-based event
//! handler. This appears to caused by an effort to allow for callback-style
//! events and access to OS-specific event data.

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

pub enum Event<'a, WindowData> {
    App(AppEvent),
    Window(Window<'a, WindowData>, WindowEvent),
}

pub enum AppEvent {
    Start,
    Suspend,
    Resume,
    Stop,
    LowMemory,
    PowerSource(PowerSource),
    MonitorState(MonitorState),
    PowerPreference(PowerPreference),
}

#[derive(Clone, Copy, Debug)]
pub enum WindowEvent {
    Activate,
    Deactivate,
    DragResize(bool),
    Resize(WindowExtent),
    DpiChange(DpiScale, WindowExtent),
    CloseRequest,
    Shown,
    Hidden,
    Maximized,
    Minimized,
    Restored,
    Move(WindowPoint),
    Wake,
    Repaint(PaintReason),
    Destroy,
    Key(KeyCode, ButtonState, ModifierKeys),
    MouseButton(MouseButton, ButtonState, WindowPoint, ModifierKeys),
    MouseScrolled(f32, ScrollAxis, ModifierKeys),
    PointerMoved(WindowPoint),
    PointerEntered(WindowPoint),
    PointerLeft,
}

pub trait Handler<WindowData> {
    fn handle(&mut self, event_loop: &ActiveEventLoop<WindowData>, event: Event<WindowData>);
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

    pub fn run<WindowData, H: Handler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), EventLoopError> {
        self.event_loop.run(event_handler)
    }
}
