use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, ShowWindow, SW_HIDE, SW_SHOW, WM_APP},
};

use crate::{
    application::AppContext,
    graphics::{FramesPerSecond, RefreshRate},
    math::{Point, Scale, Size},
    util::AcRead,
    window::{Input, Window, WindowEvent, WindowPoint, WindowSize},
    WindowEventHandlerConstructor,
};

pub(super) const UM_DESTROY_WINDOW: u32 = WM_APP;
pub(super) const UM_ANIM_REQUEST: u32 = WM_APP + 1;
pub(super) const WINDOWS_DEFAULT_DPI: u16 = 96;

pub(super) enum UiEvent {
    NewWindow(u32, HWND, &'static WindowEventHandlerConstructor),
    DestroyWindow(u32),
    Shutdown,
    Input(u32, Input),
    Window(u32, WindowEvent),
    ControlEvent(u32, Control),
}

#[derive(Clone, Copy, Debug)]
pub enum Control {
    AnimationFreq(FramesPerSecond),
    Repaint,
}

#[derive(Debug)]
pub struct WindowState {
    pub size: WindowSize,

    pub is_visible: bool,
    pub is_resizing: bool,

    pub pointer_location: Option<WindowPoint>,

    pub actual_refresh_rate: FramesPerSecond,
    pub requested_refresh_rate: FramesPerSecond,
}

/// The number of windows that exist or that have been queued for creation.
///
/// This is so that we don't go over the MAX_WINDOWS limit and can provide
/// useful errors at the call site of `spawn_window`.
pub static NUM_SPAWNED: Mutex<u32> = Mutex::new(0);

pub struct WindowImpl {
    hwnd: HWND,
    state: AcRead<'static, Option<WindowState>>,
    context: AppContext,
}

impl WindowImpl {
    pub(super) fn new(
        hwnd: HWND,
        state: AcRead<'static, Option<WindowState>>,
        context: AppContext,
    ) -> Self {
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
        let rate = self.state.read().as_ref().unwrap().actual_refresh_rate;

        RefreshRate {
            min: FramesPerSecond::ZERO,
            max: self.context.inner.composition_rate(),
            now: rate,
        }
    }

    pub fn size(&self) -> Size<Window> {
        let size = self.state.read().as_ref().unwrap().size;
        Size::new(size.width as f32, size.height as f32)
    }

    pub fn scale(&self) -> Scale<Window, Window> {
        let dpi = self.state.read().as_ref().unwrap().size.dpi;

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
        let p = self.state.read().as_ref().unwrap().pointer_location?;
        Some(Point::new(p.x as f32, p.y as f32))
    }
}
