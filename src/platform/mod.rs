use slotmap::new_key_type;

use crate::{
    frame::{FrameId, FramesPerSecond},
    EventHandler, WindowError, WindowSpec,
};

#[cfg(any(target_os = "windows", doc))]
pub mod win32;

#[cfg(any(target_os = "windows", doc))]
pub mod dx12;

new_key_type! {
    pub struct WindowId;
}

/// Additional methods that may be implemented by a platfowm window handler.
pub trait PlatformEventHandler: EventHandler {
    /// Must call `EventHandler::on_repaint()`.
    fn on_os_repaint(&mut self);

    /// Must call `EventHandler::on_repaint()`.
    fn on_client_repaint(&mut self);

    // Must call `EventHandler::on_repaint()` to draw on vsync.
    fn on_vsync(&mut self, frame_id: FrameId, rate: Option<FramesPerSecond>);

    fn on_composition_rate_change(&mut self, frame_id: FrameId, rate: FramesPerSecond);
}

// all window handling happens within the event loop's control anyway.
// particulars of threading beheavior are not as important.

// win32 requires one thread for the event loop, and one more for the vsync
// clock. All windows can use the same event loop, which removes the need for
// messaging. The current implementation creates one event loop per thread, each
// with a capacity of up to 10,000 events. That's not necessary. Rendering UIs
// should not take very long at all, and being able to submit multiple windows'
// worth of drawing at once allows the GPU to work on all of them at the same
// time.

// Application::new().create_window().create_window().run();

// only use enum on linux, no need to be object safe
pub trait EventLoop<WindowStorage: PlatformEventHandler>: WindowSystem<WindowStorage> {
    fn new() -> Self;

    fn run(&mut self);
}

pub trait WindowSystem<WindowStorage: PlatformEventHandler> {
    type WindowHandle: Copy;

    fn create_window<P, F>(&mut self, spec: WindowSpec, constructor: F) -> Result<(), WindowError>
    where
        P: PlatformEventHandler + Into<WindowStorage>,
        F: FnMut(Self::WindowHandle) -> P;
}
