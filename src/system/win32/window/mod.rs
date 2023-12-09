mod event_thread;
mod handler_thread;

use std::sync::Arc;

use parking_lot::RwLock;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{PostMessageW, ShowWindow, SW_HIDE, SW_SHOW, WM_USER},
};

use crate::{
    animation::{AnimationFrequency, PresentTiming},
    input::{Axis, ButtonState, MouseButton},
    math::{Point, Scale, Size},
    window::{WindowEventHandler, WindowSpec},
};

use super::AppContext;

const UM_DESTROY_WINDOW: u32 = WM_USER;

#[derive(Debug)]
pub enum Event {
    Create(HWND),
    CloseRequest,
    Destroy,
    Visible(bool),
    BeginResize,
    Resize { width: u32, height: u32, scale: f64 },
    EndResize,
    Repaint(PresentTiming),
    MouseButton(MouseButton, ButtonState, (i16, i16)),
    PointerMove((i16, i16)),
    PointerLeave,
    Scroll(Axis, f32),
}

#[derive(Default)]
struct SharedState {
    size: Size<crate::window::Window>,

    /// The most recent position of the cursor, or `None` if the cursor is not
    /// in the window's client area.
    pointer_location: Option<Point<crate::window::Window>>,
}

pub struct Window {
    hwnd: HWND,
    context: crate::application::AppContext,
    shared_state: Arc<RwLock<SharedState>>,
}

impl Window {
    pub fn app(&self) -> &crate::application::AppContext {
        &self.context
    }

    pub fn close(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DESTROY_WINDOW, None, None) }.unwrap();
    }

    pub fn begin_animation(&mut self, _freq: Option<AnimationFrequency>) {
        todo!()
    }

    pub fn end_animation(&mut self) {
        todo!()
    }

    pub fn default_animation_frequency(&self) -> AnimationFrequency {
        todo!()
    }

    pub fn size(&self) -> Size<crate::window::Window> {
        self.shared_state.read().size
    }

    pub fn scale(&self) -> Scale<crate::window::Window, crate::window::Window> {
        todo!()
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Option<Point<crate::window::Window>> {
        self.shared_state.read().pointer_location
    }
}

/// Creates a window and the two threads used to handle it.
///
/// It may seem excessive to use two threads to handle a single window, but it
/// gives us a few advantages:
///
/// 1. Assigning one thread per window allows the runtime of event processing
///    and drawing to be handled in parallel on a multi-core processor. This
///    allows slow windows to update at their own pace without slowing down
///    faster windows
/// 2. Using a second thread per window gives us more control over redraw events
///    while in the modal event loop. This means that animations don't freeze up
///    or stutter while the user is resizing.
pub fn spawn_window<W, F>(context: AppContext, spec: WindowSpec, constructor: F)
where
    W: WindowEventHandler + 'static,
    F: FnMut(crate::window::Window) -> W + Send + 'static,
{
    let (evt_send, evt_recv) = std::sync::mpsc::channel();

    event_thread::spawn(spec, evt_send);
    handler_thread::spawn(context, constructor, evt_recv);
}
