use std::sync::Arc;

use parking_lot::RwLock;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, ShowWindow, SW_HIDE, SW_SHOW, WM_APP},
};

use crate::{
    application::AppContext,
    frame::{FrameId, FramesPerSecond, RedrawRequest, RefreshRate},
    math::{Point, Scale, Size},
    window::{Window, WindowPoint, WindowSize},
};

pub(super) const UM_DESTROY_WINDOW: u32 = WM_APP;
pub(super) const UM_REDRAW_REQUEST: u32 = WM_APP + 1;
pub(super) const WINDOWS_DEFAULT_DPI: u16 = 96;

#[derive(Clone, Copy, Debug)]
pub enum Control {
    /// The client applicaton has requested that the window be repainted.
    Redraw(RedrawRequest),
    /// The OS has requested that the window be repainted.
    OsRepaint,
}

#[derive(Debug, Default)]
pub struct WindowState {
    pub size: WindowSize,

    pub is_visible: bool,
    pub is_resizing: bool,

    pub pointer_location: Option<WindowPoint>,

    pub composition_rate: FramesPerSecond,
    pub actual_refresh_rate: FramesPerSecond,
    pub requested_refresh_rate: FramesPerSecond,
}

pub struct WindowImpl {
    hwnd: HWND,
    state: Arc<RwLock<WindowState>>,
    context: AppContext,
}

impl WindowImpl {
    pub(super) fn new(hwnd: HWND, state: Arc<RwLock<WindowState>>, context: AppContext) -> Self {
        Self {
            hwnd,
            state,
            context,
        }
    }

    pub fn app(&self) -> &AppContext {
        &self.context
    }

    pub fn close(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DESTROY_WINDOW, None, None) }.unwrap();
    }

    pub fn request_redraw(&self, request: RedrawRequest) {
        post_redraw_request(self.hwnd, request);
    }

    pub fn refresh_rate(&self) -> RefreshRate {
        let state = self.state.read();

        RefreshRate {
            min: FramesPerSecond::ZERO,
            max: state.composition_rate,
            now: state.actual_refresh_rate,
        }
    }

    pub fn size(&self) -> Size<Window> {
        let size = self.state.read().size;
        Size::new(size.width as f32, size.height as f32)
    }

    pub fn scale(&self) -> Scale<Window, Window> {
        let dpi = self.state.read().size.dpi;

        Scale::new(
            dpi as f32 / WINDOWS_DEFAULT_DPI as f32,
            dpi as f32 / WINDOWS_DEFAULT_DPI as f32,
        )
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Option<Point<Window>> {
        let p = self.state.read().pointer_location?;
        Some(Point::new(p.x as f32, p.y as f32))
    }
}

fn post_redraw_request(hwnd: HWND, request: RedrawRequest) {
    const _: () = assert!(std::mem::size_of::<usize>() == std::mem::size_of::<FrameId>());
    const _: () = assert!(std::mem::size_of::<usize>() == std::mem::size_of::<FramesPerSecond>());

    let (lp, wp) = match request {
        RedrawRequest::Idle => (0, 0),
        RedrawRequest::Once => (1, 0),
        RedrawRequest::AtFrame(id) => (2, id.0),
        RedrawRequest::AtFrameRate(rate) => (3, rate.0.to_bits()),
    };

    unsafe { PostMessageW(hwnd, UM_REDRAW_REQUEST, WPARAM(wp as _), LPARAM(lp)) }.unwrap();
}

pub(super) fn extract_redraw_request(wparam: WPARAM, lparam: LPARAM) -> RedrawRequest {
    const _: () = assert!(std::mem::size_of::<usize>() == std::mem::size_of::<FrameId>());
    const _: () = assert!(std::mem::size_of::<usize>() == std::mem::size_of::<FramesPerSecond>());

    match lparam.0 {
        0 => RedrawRequest::Idle,
        1 => RedrawRequest::Once,
        2 => RedrawRequest::AtFrame(FrameId(wparam.0 as _)),
        3 => RedrawRequest::AtFrameRate(FramesPerSecond(f64::from_bits(wparam.0 as _))),
        _ => unreachable!(),
    }
}
