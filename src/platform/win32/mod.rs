//! Windows platform implementation.
//!
//! This implementation creates windows and processes their messages on a single
//! thread. This means that each window handler must take care to be fast and
//! not block since every window shares the same thread. This greatly simplifies
//! the implementation and defers any complexity to higher levels of the
//! application.

mod application;
mod vsync;
mod window;

use std::cell::{Cell, RefCell};

pub use application::{AppContextImpl, ApplicationImpl};
use arrayvec::ArrayVec;
pub(crate) use vsync::VSyncRequest;
pub use window::WindowImpl;

use lazy_static::lazy_static;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::HBRUSH,
        System::{
            LibraryLoader::GetModuleHandleW,
            Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
        },
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcA, DispatchMessageW, GetMessageW,
            LoadCursorW, PeekMessageW, RegisterClassExW, SetWindowLongPtrW, TranslateMessage,
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON, HMENU, IDC_ARROW, MSG,
            PM_NOREMOVE, WM_TIMER, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::{
    geometry::Size,
    limits::{MAX_WINDOWS, MAX_WINDOW_TITLE_LENGTH},
    PhysicalPixel, WindowError, WindowSpec,
};

use super::PlatformEventHandler;

const CLASS_NAME: PCWSTR = w!("plinth_window_class");
const DEFAULT_DPI: u16 = 96;

lazy_static! {
    static ref QPF_FREQUENCY: i64 = {
        let mut freq = 0;
        unsafe { QueryPerformanceFrequency(&mut freq) }.unwrap();
        freq
    };
}

pub fn present_time_now() -> f64 {
    let mut time = 0;
    unsafe { QueryPerformanceCounter(&mut time) }.unwrap();
    (time / *QPF_FREQUENCY) as f64
}

pub fn present_time_from_ticks(ticks: i64) -> f64 {
    (ticks / *QPF_FREQUENCY) as f64
}

struct WindowState {
    hwnd: HWND,
    size: Cell<(Size<u16, PhysicalPixel>, u16)>,
    has_pointer: Cell<bool>,
}

#[derive(Default)]
struct CallbackState {
    in_modal: Cell<bool>,
    resize_target: Cell<HWND>,
    windows: ArrayVec<WindowState, MAX_WINDOWS>,
    callbacks: ArrayVec<*const RefCell<dyn PlatformEventHandler>, MAX_WINDOWS>,
}

/// Win32 event loop.
///
/// All windows are created on the same event loop for simplicity's sake. Since
/// most applications use only a single window and almost exclusively less than
/// 4, this is not a problem.
pub struct EventLoop<WindowStorage: PlatformEventHandler> {
    wndclass: PCWSTR,
    handlers: ArrayVec<(HWND, RefCell<WindowStorage>), MAX_WINDOWS>,
    state: CallbackState,
}

impl<WindowStorage: PlatformEventHandler> crate::platform::EventLoop<WindowStorage>
    for EventLoop<WindowStorage>
{
    fn new() -> Self {
        let wndclass = register_wndclass();

        let handlers = ArrayVec::new();
        let state = CallbackState::default();

        Self {
            wndclass,
            handlers,
            state,
        }
    }

    fn run(&mut self) {
        let mut msg = MSG::default();

        assert_eq!(self.handlers.len(), self.state.windows.len());
        assert!(self.state.callbacks.is_empty());

        // initialize window user pointers
        for i in 0..self.handlers.len() {
            let (hwnd, handler) = &self.handlers[i];
            self.state.callbacks.push(handler as *const _);
            unsafe { SetWindowLongPtrW(*hwnd, GWLP_USERDATA, &self.state as *const _ as isize) };
        }

        loop {
            // To clear timers. If the event loop is busy, it may never get to
            // these, since they are auto-generated when the queue is empty.
            unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

            match unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 {
                -1 => {
                    panic!(
                        "Failed to get message, error code: {}",
                        unsafe { GetLastError() }.unwrap_err().message()
                    );
                }
                0 => break,
                1 => unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
                _ => unreachable!(),
            }

            // handle any pending window creates
        }
    }
}

impl<WindowStorage: PlatformEventHandler> crate::platform::WindowSystem<WindowStorage>
    for EventLoop<WindowStorage>
{
    type WindowHandle = HWND;

    fn create_window<P, F>(
        &mut self,
        spec: crate::WindowSpec,
        mut constructor: F,
    ) -> Result<(), crate::WindowError>
    where
        P: super::PlatformEventHandler + Into<WindowStorage>,
        F: FnMut(Self::WindowHandle) -> P,
    {
        if self.handlers.is_full() {
            return Err(WindowError::TooManyWindows);
        }

        let hwnd = create_window(self.wndclass, &spec).unwrap();

        let Err(index) = self.handlers.binary_search_by_key(&hwnd.0, |s| s.0 .0) else {
            unreachable!()
        };

        let handler = constructor(hwnd);
        self.handlers
            .insert(index, (hwnd, RefCell::new(handler.into())));

        self.state.windows.insert(
            index,
            WindowState {
                hwnd,
                size: Cell::new((spec.size, DEFAULT_DPI)),
                has_pointer: Cell::new(false),
            },
        );

        Ok(())
    }
}

fn register_wndclass() -> PCWSTR {
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc_trampoline),
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

    PCWSTR(atom as usize as *const _)
}

fn create_window(wndclass: PCWSTR, spec: &WindowSpec) -> Result<HWND, ()> {
    let title = {
        let mut title = spec
            .title
            .encode_utf16()
            .collect::<ArrayVec<_, { MAX_WINDOW_TITLE_LENGTH + 1 }>>();

        assert!(title.len() <= MAX_WINDOW_TITLE_LENGTH);
        title.push(0);
        title
    };

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: i32::from(spec.size.width),
        bottom: i32::from(spec.size.height),
    };

    unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) }.unwrap();

    let hwnd = unsafe {
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
    };

    Ok(hwnd)
}

pub fn set_userdata(hwnd: HWND, data: *const CallbackState, index: usize) {
    todo!()
}

pub fn get_userdata(hwnd: HWND) -> Option<(*const CallbackState, usize)> {
    todo!()
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcA(hwnd, msg, wparam, lparam)
}
