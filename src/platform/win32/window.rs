use std::sync::atomic::{AtomicBool, AtomicU32};

use arrayvec::ArrayVec;
use parking_lot::{Mutex, RwLock};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::HBRUSH,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, LoadCursorW, PostMessageW, RegisterClassExW,
            ShowWindow, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HICON, HMENU, IDC_ARROW, SW_HIDE,
            SW_SHOW, WM_APP, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::{
    application::AppContext,
    graphics::{FramesPerSecond, RefreshRate},
    limits::{MAX_TITLE_LENGTH, MAX_WINDOWS},
    math::{Point, Scale, Size},
    window::{Input, Window, WindowEvent, WindowEventHandler, WindowPoint, WindowSize},
    WindowSpec,
};

pub(super) const UM_DESTROY_WINDOW: u32 = WM_APP;
pub(super) const UM_ANIM_REQUEST: u32 = WM_APP + 1;

const CLASS_NAME: PCWSTR = w!("plinth_window_class");
const WINDOWS_DEFAULT_DPI: u16 = 96;

pub(super) enum UiEvent {
    NewWindow(u32, HWND, Box<dyn WindowEventHandler>),
    Shutdown,
    Input(u32, Input),
    Window(u32, WindowEvent),
    ControlEvent(u32, Control),
}

pub enum Control {
    AnimationFreq(FramesPerSecond),
    Repaint,
}

#[derive(Debug)]
pub struct WindowState {
    pub size: RwLock<WindowSize>,

    pub is_visible: AtomicBool,
    pub is_resizing: AtomicBool,

    pub pointer_location: Mutex<Option<WindowPoint>>,

    /// Refresh rate stored as an f32, bits transmuted for storage.
    pub actual_refresh_rate: AtomicU32,

    /// Refresh rate stored as an f32, bits transmuted for storage.
    pub requested_refresh_rate: AtomicU32,
}

const fn default_window_state() -> WindowState {
    WindowState {
        size: RwLock::new(WindowSize {
            width: 0,
            height: 0,
            dpi: WINDOWS_DEFAULT_DPI,
        }),
        is_visible: AtomicBool::new(true),
        is_resizing: AtomicBool::new(false),
        pointer_location: Mutex::new(None),
        actual_refresh_rate: AtomicU32::new(0),
        requested_refresh_rate: AtomicU32::new(0),
    }
}

pub fn reset_window_state(window: &WindowState) {
    *window.size.write() = WindowSize {
        width: 0,
        height: 0,
        dpi: WINDOWS_DEFAULT_DPI,
    };
    window
        .is_visible
        .store(false, std::sync::atomic::Ordering::Relaxed);
    window
        .is_resizing
        .store(false, std::sync::atomic::Ordering::Relaxed);
    *window.pointer_location.lock() = None;
    window
        .actual_refresh_rate
        .store(0, std::sync::atomic::Ordering::Relaxed);
    window
        .requested_refresh_rate
        .store(0, std::sync::atomic::Ordering::Relaxed);
}

#[repr(align(64))]
pub struct Pad64<T>(T);

impl<T> std::ops::Deref for Pad64<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Pad64<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The number of windows that exist or that have been queued for creation.
///
/// This is so that we don't go over the MAX_WINDOWS limit and can provide
/// useful errors at the call site of `spawn_window`.
pub static NUM_SPAWNED: Mutex<u32> = Mutex::new(0);

pub static WINDOWS: [Pad64<WindowState>; MAX_WINDOWS] = [
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
    Pad64(default_window_state()),
];

const _: () = assert!(std::mem::size_of::<Pad64<WindowState>>() == 64);

pub struct WindowOccupancy(i32);

impl WindowOccupancy {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_occupied(&self, index: u32) -> bool {
        self.0 & (1 << index) != 0
    }

    pub fn next_occupied(&mut self) -> Option<u32> {
        let inverted = !self.0;
        if inverted != 0 {
            let index = inverted.trailing_zeros();
            self.0 |= 1 << index;
            Some(index)
        } else {
            None
        }
    }

    pub fn set_occupied(&mut self, index: u32, occupied: bool) {
        if occupied {
            self.0 |= 1 << index;
        } else {
            self.0 &= !(1 << index);
        }
    }
}

pub struct WindowImpl {
    pub(super) hwnd: HWND,
    pub(super) index: u32,
    pub(super) context: AppContext,
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
        let rate = WINDOWS[self.index as usize]
            .actual_refresh_rate
            .load(std::sync::atomic::Ordering::Relaxed);

        // RefreshRate(f32::from_bits(rate))
        todo!()
    }

    pub fn size(&self) -> Size<Window> {
        let size = *WINDOWS[self.index as usize].size.read();
        Size::new(size.width as f32, size.height as f32)
    }

    pub fn scale(&self) -> Scale<Window, Window> {
        todo!()
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Option<Point<Window>> {
        let p = *WINDOWS[self.index as usize].pointer_location.lock();
        p.map(|p| Point::new(p.x as f32, p.y as f32))
    }
}

pub(super) fn register_wndclass(
    wndproc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> PCWSTR {
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: unsafe { GetModuleHandleW(None) }.unwrap().into(),
        hIcon: HICON::default(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: CLASS_NAME,
        hIconSm: HICON::default(),
    };

    let atom = unsafe { RegisterClassExW(&class) };

    if atom == 0 {
        unsafe { GetLastError() }.expect("Failed to register window class");
    } else {
        tracing::info!("Registered window class");
    }

    PCWSTR(atom as usize as *const u16)
}

pub(super) fn create_window(wndclass: PCWSTR, spec: &WindowSpec) -> HWND {
    let title = {
        let mut title = spec
            .title
            .encode_utf16()
            .collect::<ArrayVec<_, { MAX_TITLE_LENGTH + 1 }>>();

        assert!(title.len() <= MAX_TITLE_LENGTH);
        title.push(0);
        title
    };

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: i32::try_from(spec.size.width as i64).unwrap(), // cast to i64 to preserve f64's 48 bits of precision
        bottom: i32::try_from(spec.size.height as i64).unwrap(), // ditto
    };

    unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) }.unwrap();

    // SAFETY: This gets passed into CreateWindowExW, which must not
    // persist the pointer beyond WM_CREATE, or it will produce a
    // dangling pointer.
    unsafe {
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            // Use the atom for later comparison. This way we don't have to
            // compare c-style strings.
            wndclass,
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right,
            rect.bottom,
            HWND::default(),
            HMENU::default(),
            HMODULE::default(),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_occupancy() {
        let mut occupancy = WindowOccupancy::new();

        for i in 0..MAX_WINDOWS {
            assert_eq!(occupancy.next_occupied(), Some(i as u32));
        }

        assert_eq!(occupancy.next_occupied(), None);

        occupancy.set_occupied(0, false);
        assert!(!occupancy.is_occupied(0));
        assert_eq!(occupancy.next_occupied(), Some(0));

        occupancy.set_occupied(12, false);
        assert!(!occupancy.is_occupied(12));
        occupancy.set_occupied(12, true);
        assert!(occupancy.is_occupied(12));
    }
}
