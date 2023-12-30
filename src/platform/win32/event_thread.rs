//! The Windows message pump.
//!
//! The message pump is located on its own thread and events are sent to an
//! event handler via a channel.

use std::{
    ptr::addr_of_mut,
    sync::{mpsc::Sender, OnceLock},
};

use arrayvec::ArrayVec;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::WM_MOUSELEAVE,
            Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
            WindowsAndMessaging::{
                AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW,
                PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON, HMENU, IDC_ARROW, MSG,
                PM_NOREMOVE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
                WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SHOWWINDOW,
                WM_TIMER, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP,
                WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use crate::{
    graphics::FramesPerSecond,
    input::{Axis, ButtonState, MouseButton},
    limits::MAX_TITLE_LENGTH,
    window::WindowSpec,
};

use super::window::{UM_ANIM_REQUEST, UM_DESTROY_WINDOW};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

struct State {
    sender: Sender<Event>,
    width: u32,
    height: u32,
    is_size_move: bool,
    is_resizing: bool,
    pointer_in_client_area: bool,
}

/// Spawns a new thread and creates a window and its event loop on it.
///
/// The window gets sent to the event handler thread via a channel.
pub(super) fn spawn(spec: WindowSpec, sender: Sender<Event>) {
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
        sender
            .send(Event::SetAnimationFrequency(
                spec.refresh_rate.unwrap_or_default(),
            ))
            .unwrap();

        let mut state = State {
            sender,
            width: 0,
            height: 0,
            is_size_move: false,
            is_resizing: false,
            pointer_in_client_area: false,
        };

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, addr_of_mut!(state) as _) };

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

    if state == 0 {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    } else {
        let state = state as *mut State;
        wndproc(&mut *state, hwnd, msg, wparam, lparam)
    }
}

fn mouse_coords(lparam: LPARAM) -> (i16, i16) {
    let x = (lparam.0 & 0xffff) as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as i16;
    (x, y)
}

#[tracing::instrument(skip(state))]
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
        UM_ANIM_REQUEST => {
            let freq = f64::from_bits(wparam.0 as u64);
            state
                .sender
                .send(Event::SetAnimationFrequency(FramesPerSecond(freq)))
                .unwrap();
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            state.sender.send(Event::Repaint).unwrap();

            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state.pointer_in_client_area {
                state.pointer_in_client_area = true;

                unsafe {
                    TrackMouseEvent(&mut TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    })
                }
                .unwrap();
            }

            state
                .sender
                .send(Event::PointerMove(mouse_coords(lparam)))
                .unwrap();

            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            state.pointer_in_client_area = false;
            state.sender.send(Event::PointerLeave).unwrap();
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            state.sender.send(Event::Scroll(Axis::Y, delta)).unwrap();
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            state.sender.send(Event::Scroll(Axis::X, delta)).unwrap();
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Left,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Left,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Right,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Right,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Middle,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            state
                .sender
                .send(Event::MouseButton(
                    MouseButton::Middle,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ))
                .unwrap();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
