use std::{
    cell::Cell,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TryRecvError},
    },
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
    limits::{MAX_TITLE_LENGTH, MAX_WINDOWS},
    time::FramesPerSecond,
    util::BitMap32,
    window::{Axis, ButtonState, Input, MouseButton, WindowEvent, WindowPoint, WindowSize},
    WindowSpec,
};

use super::{
    application::AppMessage,
    window::{
        Control, UiEvent, NUM_SPAWNED, UM_ANIM_REQUEST, UM_DESTROY_WINDOW, WINDOWS_DEFAULT_DPI,
    },
};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

static RUNNING: AtomicBool = AtomicBool::new(false);

struct EventState {
    index: Cell<u32>,
    sender: Sender<UiEvent>,
    width: Cell<u16>,
    height: Cell<u16>,
    pointer_in_window: Cell<bool>,
    is_in_size_move: Cell<bool>,
}

pub fn run_event_loop(receiver: &Receiver<AppMessage>, sender: &Sender<UiEvent>) {
    assert!(
        RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok(),
        "the event loop can only be run by a single thread at a time"
    );

    let wndclass = register_wndclass(wndproc_trampoline);

    let mut occupancy = BitMap32::new();

    let mut event_state: [MaybeUninit<EventState>; MAX_WINDOWS] =
        [(); MAX_WINDOWS].map(|_| MaybeUninit::uninit());

    let mut msg = MSG::default();
    loop {
        unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

        match receiver.try_recv() {
            Ok(AppMessage::CreateWindow(spec, constructor)) => {
                assert!(i32::BITS as usize <= MAX_WINDOWS);

                let index = occupancy.next_unset().unwrap();

                let hwnd = create_window(wndclass, &spec);

                if spec.visible {
                    unsafe { ShowWindow(hwnd, SW_SHOW) };
                }

                event_state[index as usize] = MaybeUninit::new(EventState {
                    index: Cell::new(index),
                    sender: sender.clone(),
                    width: Cell::new(spec.size.width as u16),
                    height: Cell::new(spec.size.height as u16),
                    pointer_in_window: Cell::new(false),
                    is_in_size_move: Cell::new(false),
                });

                unsafe {
                    SetWindowLongPtrW(
                        hwnd,
                        GWLP_USERDATA,
                        &event_state[index as usize] as *const _ as _,
                    );
                }

                sender
                    .send(UiEvent::NewWindow(index, hwnd, constructor))
                    .unwrap();

                if let Some(refresh_rate) = spec.refresh_rate {
                    sender
                        .send(UiEvent::ControlEvent(
                            index,
                            Control::AnimationFreq(refresh_rate),
                        ))
                        .unwrap();
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return,
        }

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
            _ => {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                if msg.message == WM_DESTROY {
                    let state: *const EventState =
                        unsafe { GetWindowLongPtrW(msg.hwnd, GWLP_USERDATA) } as *const _;
                    assert!(
                        !state.is_null(),
                        "WM_DESTROY should only be sent to windows created by us"
                    );

                    occupancy.set(unsafe { (*state).index.get() }, false);
                }
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
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const EventState;

    // :udata_index
    if state.is_null() {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    } else {
        let state = &*state;
        wndproc(state, hwnd, msg, wparam, lparam)
    }
}

fn mouse_coords(lparam: LPARAM) -> WindowPoint {
    let x = (lparam.0 & 0xffff) as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as i16;
    WindowPoint { x, y }
}

fn wndproc(state: &EventState, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let index = state.index.get();
    let sender = &state.sender;

    let send = |event| sender.send(event).unwrap();

    match msg {
        WM_CLOSE => {
            send(UiEvent::Window(index, WindowEvent::CloseRequest));
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            LRESULT(0)
        }
        WM_DESTROY => {
            send(UiEvent::DestroyWindow(index));

            let quit = {
                let mut num_spawned = NUM_SPAWNED.lock();
                *num_spawned -= 1;
                *num_spawned == 0
            };

            if quit {
                unsafe { PostQuitMessage(0) };
                send(UiEvent::Shutdown);
            }

            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            send(UiEvent::Window(index, WindowEvent::Visible(wparam.0 != 0)));
            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            state.is_in_size_move.set(true);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            state.is_in_size_move.set(false);
            send(UiEvent::Window(index, WindowEvent::EndResize));
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let (width, height) = unsafe {
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect).unwrap();

                ((rect.right - rect.left) as _, (rect.bottom - rect.top) as _)
            };

            // we don't care about window position, so ignore it

            let is_resizing = width != state.width.get() || height != state.height.get();

            if is_resizing {
                if state.is_in_size_move.get() {
                    sender
                        .send(UiEvent::Window(index, WindowEvent::BeginResize))
                        .unwrap();
                }

                send(UiEvent::Window(
                    index,
                    WindowEvent::Resize(WindowSize {
                        width,
                        height,
                        dpi: WINDOWS_DEFAULT_DPI,
                    }),
                ));
            }

            LRESULT(0)
        }
        UM_ANIM_REQUEST => {
            let freq = f64::from_bits(wparam.0 as u64);

            send(UiEvent::ControlEvent(
                index,
                Control::AnimationFreq(FramesPerSecond(freq)),
            ));

            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            send(UiEvent::ControlEvent(index, Control::Repaint));

            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state.pointer_in_window.get() {
                unsafe {
                    TrackMouseEvent(&mut TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    })
                }
                .unwrap();

                state.pointer_in_window.set(true);
            }

            let mouse_coords = mouse_coords(lparam);
            send(UiEvent::Input(index, Input::PointerMove(mouse_coords)));
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            send(UiEvent::Input(index, Input::PointerLeave));
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            send(UiEvent::Input(index, Input::Scroll(Axis::Y, delta)));
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            send(UiEvent::Input(index, Input::Scroll(Axis::X, delta)));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Left,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Left,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Right,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Right,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Middle,
                    ButtonState::Pressed,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            send(UiEvent::Input(
                index,
                Input::MouseButton(
                    MouseButton::Middle,
                    ButtonState::Released,
                    mouse_coords(lparam),
                ),
            ));
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn register_wndclass(
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

fn create_window(wndclass: PCWSTR, spec: &WindowSpec) -> HWND {
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
