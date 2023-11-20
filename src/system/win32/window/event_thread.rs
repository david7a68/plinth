use std::{
    sync::{mpsc::Sender, OnceLock},
    time::Instant,
};

use arrayvec::ArrayVec;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW,
            PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON, HMENU, IDC_ARROW, MSG,
            PM_NOREMOVE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
            WM_PAINT, WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED, WNDCLASSEXW,
            WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::{
    animation::PresentTiming,
    window::{WindowSpec, MAX_TITLE_LENGTH},
};

use super::{Event, UM_DESTROY_WINDOW};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

struct State {
    sender: Sender<Event>,
    width: u32,
    height: u32,
    is_size_move: bool,
    is_resizing: bool,
}

/// Spawns a new thread and creates a window and its event loop on it.
///
/// The window gets sent to the event handler thread via a channel.
pub fn spawn(spec: WindowSpec, sender: Sender<Event>) {
    std::thread::spawn(move || {
        let wndclass = ensure_wndclass_registered();

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

        sender.send(Event::Create(hwnd)).unwrap();

        let mut state = State {
            sender,
            width: 0,
            height: 0,
            is_size_move: false,
            is_resizing: false,
        };

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut state as *mut _ as _) };

        if spec.visible {
            unsafe { ShowWindow(hwnd, SW_SHOW) };
        }

        let mut msg = MSG::default();
        loop {
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

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    });
}

fn ensure_wndclass_registered() -> PCWSTR {
    static WND_CLASS_ATOM: OnceLock<u16> = OnceLock::new();

    WND_CLASS_ATOM.get_or_init(|| {
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

        atom
    });
    PCWSTR(*WND_CLASS_ATOM.get().unwrap() as usize as *const _)
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if state != 0 {
        let state = state as *mut State;
        wndproc(&mut *state, hwnd, msg, wparam, lparam)
    } else {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

fn wndproc(state: &mut State, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            state.sender.send(Event::CloseRequest).unwrap();
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            LRESULT(0)
        }
        WM_DESTROY => {
            state.sender.send(Event::Destroy).unwrap();
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            state.sender.send(Event::Visible(wparam.0 != 0)).unwrap();
            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            state.is_size_move = true;
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            state.is_size_move = false;

            if state.is_resizing {
                state.is_resizing = false;
                state.sender.send(Event::EndResize).unwrap();
            }

            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let (width, height) = unsafe {
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect).unwrap();

                ((rect.right - rect.left) as _, (rect.bottom - rect.top) as _)
            };

            // we don't care about window position, so ignore it

            if width != state.width || height != state.height {
                if state.is_size_move && !state.is_resizing {
                    state.is_resizing = true;
                    state.sender.send(Event::BeginResize).unwrap();
                }

                state.width = width;
                state.height = height;

                state
                    .sender
                    .send(Event::Resize {
                        width,
                        height,
                        scale: 1.0,
                    })
                    .unwrap();
            }

            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            state
                .sender
                .send(Event::Repaint(PresentTiming {
                    next_frame: Instant::now(),
                    last_frame: Instant::now(),
                }))
                .unwrap();

            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
