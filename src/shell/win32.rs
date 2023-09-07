use std::{
    cell::Cell,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock, Weak,
    },
};

use arrayvec::ArrayVec;
use euclid::Size2D;
use windows::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW, PostMessageW,
            PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow, ShowWindowAsync,
            TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA,
            HICON, HMENU, IDC_ARROW, MSG, PM_NOREMOVE, SW_SHOW, WM_CLOSE, WM_CREATE, WM_DESTROY,
            WM_ENTERSIZEMOVE, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_PAINT, WM_TIMER,
            WM_USER, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use super::{ScreenSpace, WindowError, WindowSpec, MAX_TITLE_LENGTH};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");
const UM_DESTROY_WINDOW: u32 = WM_USER;

static WND_CLASS_ATOM: OnceLock<u16> = OnceLock::new(); // TODO: make thread local?
static NUM_WINDOWS: AtomicU64 = AtomicU64::new(0);

fn wnd_class_atom_as_pcwstr() -> PCWSTR {
    PCWSTR(WND_CLASS_ATOM.get().unwrap().clone() as usize as *const _)
}

fn translate_title(title: &str) -> ArrayVec<u16, { MAX_TITLE_LENGTH + 1 }> {
    if title.len() > MAX_TITLE_LENGTH {
        tracing::warn!(
            "Window title is too long, truncating to {} characters",
            MAX_TITLE_LENGTH
        );
    }

    let mut title: ArrayVec<u16, { MAX_TITLE_LENGTH + 1 }> =
        title.encode_utf16().take(MAX_TITLE_LENGTH).collect();
    title.push(0);
    title
}

/// Tiny state machine to track window sizing state.
///
/// This is needed because `WM_ENTERSIZEMOVE` doesn't distinguish between the
/// sizing and moving modal loops. As a consequence, we have to track the state
/// ourselves in order to send `on_resize_begin` events. That event is only sent
/// on the first `WM_SIZE` after `WM_ENTERSIZEMOVE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SizingState {
    None,
    InSizeMove,
    Sizing,
}

struct WindowState {
    hwnd: Arc<OnceLock<HWND>>,
    state: super::WindowState,
    size: Cell<Size2D<u16, ScreenSpace>>,
    sizing_state: Cell<SizingState>,
}

pub(super) fn build_window(spec: WindowSpec, state: super::WindowState) -> WindowHandle {
    WND_CLASS_ATOM.get_or_init(|| {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc_trampoline),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: unsafe { GetModuleHandleW(None) }.unwrap(),
            hIcon: HICON::default(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: CLASS_NAME,
            hIconSm: HICON::default(),
        };

        let atom = unsafe { RegisterClassExW(&class) };

        if atom == 0 {
            panic!(
                "Failed to register window class, error code: {}",
                unsafe { GetLastError() }.0
            );
        } else {
            tracing::info!("Registered window class");
        }

        atom
    });

    let title = translate_title(&spec.title);

    let handle = Arc::default();
    let handle_weak = Arc::downgrade(&handle);

    let state = Box::into_raw(Box::new(WindowState {
        hwnd: handle,
        state: state,
        size: Cell::default(),
        sizing_state: Cell::new(SizingState::None),
    }));

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: spec.size.width.into(),
        bottom: spec.size.height.into(),
    };

    unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) };

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            // Use the atom for later comparison. This way we don't have to
            // compare c-style strings.
            wnd_class_atom_as_pcwstr(),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right,
            rect.bottom,
            HWND::default(),
            HMENU::default(),
            HMODULE::default(),
            Some(state.cast()),
        )
    };

    if hwnd.0 == 0 {
        panic!(
            "Failed to create window, error code: {}",
            unsafe { GetLastError() }.0
        );
    } else {
        tracing::info!("Created window");
    }

    unsafe { ShowWindow(hwnd, SW_SHOW) };

    WindowHandle { hwnd: handle_weak }
}

#[derive(Clone)]
pub struct WindowHandle {
    // Use Weak so what we know when the window has been destroyed.
    hwnd: Weak<OnceLock<HWND>>,
}

impl WindowHandle {
    pub fn hwnd(&self) -> Result<HWND, WindowError> {
        Ok(self
            .hwnd
            .upgrade()
            .ok_or(WindowError::AlreadyDestroyed)?
            .get()
            .expect("Internal error: Window not yet initialized.")
            .clone())
    }

    pub fn content_size(&self) -> Result<Size2D<u16, ScreenSpace>, WindowError> {
        let hwnd = self.hwnd()?;

        let mut client_rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut client_rect) };
        Ok(Size2D::new(client_rect.right, client_rect.bottom)
            .try_cast::<u16>()
            .expect("Window size is negative or larger than u16::MAX"))
    }

    pub fn show(&self) -> Result<(), WindowError> {
        let hwnd = self.hwnd()?;
        unsafe { ShowWindowAsync(hwnd, SW_SHOW) };
        Ok(())
    }

    pub fn destroy(&self) -> Result<(), WindowError> {
        let hwnd = self.hwnd()?;
        unsafe { PostMessageW(hwnd, UM_DESTROY_WINDOW, None, None) };
        Ok(())
    }

    pub fn request_redraw(&self) -> Result<(), WindowError> {
        let hwnd = self.hwnd()?;
        unsafe { InvalidateRect(hwnd, None, false) };
        Ok(())
    }
}

pub struct EventLoop {}

impl EventLoop {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&self) {
        loop {
            let mut msg = MSG::default();

            // Force any pending timer messages to be generated. This is in case
            // the message queue keeps getting higher priority messages faster
            // than it can process them.
            unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

            let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };

            match result.0 {
                -1 => {
                    panic!(
                        "Failed to get message, error code: {}",
                        result.ok().unwrap_err()
                    );
                }
                0 => {
                    // WM_QUIT
                    break;
                }
                _ => unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }
    }
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTW;

        if (*create_struct).lpszClass != wnd_class_atom_as_pcwstr() {
            // Compare against the class atom instead of trying to compare c strings.

            // This is not a window created by us. I have no idea how this could
            // happen, but just in case...

            tracing::warn!("Window created with unknown class name. Ignoring.");
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }

        let state = (*create_struct).lpCreateParams as *const WindowState;

        let _ = (*state).hwnd.set(hwnd);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as _);

        NUM_WINDOWS.fetch_add(1, Ordering::Release);

        tracing::debug!(
            "Window created. There are {} open windows.",
            NUM_WINDOWS.load(Ordering::Acquire)
        );
    }

    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;

    if state.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    } else {
        let r = wndproc(&*state, msg, wparam, lparam);

        if msg == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);

            let _ = Box::from_raw(state.cast_mut());

            NUM_WINDOWS.fetch_sub(1, Ordering::AcqRel);

            tracing::debug!(
                "Window destroyed. There are {} open windows.",
                NUM_WINDOWS.load(Ordering::Acquire)
            );

            if NUM_WINDOWS.load(Ordering::Acquire) == 0 {
                tracing::debug!("All windows closed, exiting event loop.");
                PostQuitMessage(0);
            }
        }

        r
    }
}

#[tracing::instrument(skip(window, wparam, lparam))]
fn wndproc(window: &WindowState, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let handle = super::WindowHandle {
                handle: WindowHandle {
                    hwnd: Arc::downgrade(&window.hwnd),
                },
            };

            window.state.on_create(handle);
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(window.hwnd.get()) };
            LRESULT(0)
        }
        WM_DESTROY => {
            window.state.on_destroy();
            LRESULT(0)
        }
        WM_CLOSE => {
            window.state.on_close_request();
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_ENTERSIZEMOVE => {
            #[cfg(debug_assertions)]
            {
                let state = window.sizing_state.get();
                assert_eq!(
                    state,
                    SizingState::None,
                    "Window entered size move state without exiting it."
                );
            }

            window.sizing_state.set(SizingState::InSizeMove);
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            #[cfg(debug_assertions)]
            {
                let state = window.sizing_state.get();
                assert!(
                    state == SizingState::InSizeMove || state == SizingState::Sizing,
                    "Window exited size move state without entering it."
                );
            }

            window.sizing_state.set(SizingState::None);
            window.state.on_resize_end();

            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let content_size = {
                let mut client_rect = RECT::default();
                unsafe {
                    GetClientRect(window.hwnd.get(), &mut client_rect);
                }
                Size2D::new(client_rect.right, client_rect.bottom)
                    .try_cast::<u16>()
                    .expect("Window size is negative or larger than u16::MAX")
            };

            if content_size != window.size.get() {
                window.size.set(content_size);

                if window.sizing_state.get() == SizingState::InSizeMove {
                    window.state.on_resize_begin();
                    window.sizing_state.set(SizingState::Sizing);
                }

                window.state.on_resize(content_size);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(window.hwnd.get(), &mut ps) };
            unsafe { EndPaint(window.hwnd.get(), &ps) };

            window.state.on_paint();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window.hwnd.get(), msg, wparam, lparam) },
    }
}
