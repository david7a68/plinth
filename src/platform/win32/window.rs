use std::sync::Arc;

use parking_lot::RwLock;
use windows::Win32::{
    Foundation::{HWND, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, ShowWindow, SW_HIDE, SW_SHOW, WM_APP},
};

use crate::{
    application::AppContext,
    graphics::{FramesPerSecond, RefreshRate},
    input::{Axis, ButtonState, MouseButton},
    math::{Point, Scale, Size},
    window::{Window, WindowEventHandler, WindowSpec},
};

use super::application::AppContextImpl;

pub(super) const UM_DESTROY_WINDOW: u32 = WM_APP;
pub(super) const UM_ANIM_REQUEST: u32 = WM_APP + 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Event {
    Create(HWND),
    CloseRequest,
    Destroy,
    Visible(bool),
    BeginResize,
    Resize { width: u32, height: u32, scale: f32 },
    EndResize,
    Repaint,
    MouseButton(MouseButton, ButtonState, (i16, i16)),
    PointerMove((i16, i16)),
    PointerLeave,
    Scroll(Axis, f32),
    SetAnimationFrequency(FramesPerSecond),
}

#[derive(Default)]
pub(super) struct SharedState {
    pub(super) size: Size<Window>,

    pub(super) is_visible: bool,

    /// The most recent position of the cursor, or `None` if the cursor is not
    /// in the window's client area.
    pub(super) pointer_location: Option<Point<Window>>,

    pub(super) refresh_rate: RefreshRate,
}

pub struct WindowImpl {
    pub(super) hwnd: HWND,
    pub(super) context: AppContext,
    pub(super) shared_state: Arc<RwLock<SharedState>>,
}

impl WindowImpl {
    pub fn app(&self) -> &AppContext {
        &self.context
    }

    pub fn close(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DESTROY_WINDOW, None, None) }.unwrap();
    }

    pub fn set_animation_frequency(&mut self, freq: FramesPerSecond) {
        unsafe {
            PostMessageW(
                self.hwnd,
                UM_ANIM_REQUEST,
                WPARAM(freq.0.to_bits() as usize),
                None,
            )
        }
        .unwrap();
    }

    pub fn refresh_rate(&self) -> RefreshRate {
        self.shared_state.read().refresh_rate
    }

    pub fn size(&self) -> Size<Window> {
        self.shared_state.read().size
    }

    pub fn scale(&self) -> Scale<Window, Window> {
        todo!()
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Option<Point<Window>> {
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
pub(super) fn spawn_window<W, F>(context: AppContextImpl, spec: WindowSpec, constructor: F)
where
    W: WindowEventHandler + 'static,
    F: FnMut(Window) -> W + Send + 'static,
{
    let (evt_send, evt_recv) = std::sync::mpsc::channel();

    super::event_thread::spawn(spec, evt_send);
    super::window_thread::spawn(context, constructor, evt_recv);
}
