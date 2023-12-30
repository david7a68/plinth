use std::{
    cell::Cell,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TryRecvError},
    },
};

use parking_lot::RwLockWriteGuard;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
    UI::{
        Controls::WM_MOUSELEAVE,
        Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
        WindowsAndMessaging::{
            DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
            GetWindowLongPtrW, PeekMessageW, PostQuitMessage, SetWindowLongPtrW, ShowWindow,
            TranslateMessage, GWLP_USERDATA, MSG, PM_NOREMOVE, SW_SHOW, WM_CLOSE, WM_DESTROY,
            WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN,
            WM_RBUTTONUP, WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED,
        },
    },
};

use crate::{
    application::AppContext,
    limits::MAX_WINDOWS,
    platform::{
        win32::window::{create_window, register_wndclass, WindowOccupancy, WINDOWS},
        WindowImpl,
    },
    time::FramesPerSecond,
    window::{Axis, ButtonState, Input, MouseButton, WindowEvent, WindowPoint, WindowSize},
    Window,
};

use super::{
    application::AppMessage,
    window::{
        reset_window_state, Control, UiEvent, WindowState, NUM_SPAWNED, UM_ANIM_REQUEST,
        UM_DESTROY_WINDOW,
    },
};

const WINDOWS_DEFAULT_DPI: u16 = 96;

static RUNNING: AtomicBool = AtomicBool::new(false);

struct EventState {
    index: Cell<u32>,
    is_in_size_move: Cell<bool>,
    sender: Sender<UiEvent>,
}

pub fn run_event_loop(
    context: &AppContext,
    receiver: &Receiver<AppMessage>,
    sender: &Sender<UiEvent>,
) {
    assert!(
        RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok(),
        "the event loop can only be run by a single thread at a time"
    );

    let wndclass = register_wndclass(wndproc_trampoline);

    // somehow need to allow WM_DESTROY to clear a bit
    let mut occupancy = WindowOccupancy::new();

    let mut event_state: [MaybeUninit<EventState>; MAX_WINDOWS] =
        [(); MAX_WINDOWS].map(|_| MaybeUninit::uninit());

    let mut msg = MSG::default();
    loop {
        unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

        match receiver.try_recv() {
            Ok(AppMessage::CreateWindow(spec, constructor)) => {
                assert!(i32::BITS as usize <= MAX_WINDOWS);

                let index = occupancy.next_occupied().unwrap();

                let hwnd = create_window(wndclass, &spec);
                let window = Window::new(WindowImpl {
                    hwnd,
                    index,
                    context: context.clone(),
                });

                let handler = constructor(window);

                if spec.visible {
                    unsafe { ShowWindow(hwnd, SW_SHOW) };
                }

                // assumption: windows are reset to default state when they are destroyed

                *WINDOWS[index as usize].size.write() = WindowSize {
                    width: spec.size.width as u16,
                    height: spec.size.height as u16,
                    dpi: WINDOWS_DEFAULT_DPI,
                };

                event_state[index as usize] = MaybeUninit::new(EventState {
                    index: Cell::new(index),
                    is_in_size_move: Cell::new(false),
                    sender: sender.clone(),
                });

                unsafe {
                    SetWindowLongPtrW(
                        hwnd,
                        GWLP_USERDATA,
                        &event_state[index as usize] as *const _ as _,
                    );
                }

                sender
                    .send(UiEvent::NewWindow(index, hwnd, handler))
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

                    occupancy.set_occupied(unsafe { (*state).index.get() }, false);
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
        let window = &WINDOWS[state.index.get() as usize];
        wndproc(window, state, hwnd, msg, wparam, lparam)
    }
}

fn mouse_coords(lparam: LPARAM) -> WindowPoint {
    let x = (lparam.0 & 0xffff) as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as i16;
    WindowPoint { x, y }
}

fn wndproc(
    window: &WindowState,
    state: &EventState,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
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
            send(UiEvent::Window(index, WindowEvent::Destroy));
            reset_window_state(window);

            let quit = {
                let mut num_spawned = NUM_SPAWNED.lock();
                *num_spawned -= 1;
                tracing::info!("num_spawned: {}", *num_spawned);
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

            let r = window.is_resizing.compare_exchange(
                true,
                false,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );

            assert_eq!(
                r,
                Ok(true),
                "only the event loop thread should be able to modify state.is_resizing"
            );

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

            let mut size = window.size.write();
            if width != size.width || height != size.height {
                if state.is_in_size_move.get() {
                    if window.is_resizing.compare_exchange(
                        false,
                        true,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ) == Ok(false)
                    {
                        sender
                            .send(UiEvent::Window(index, WindowEvent::BeginResize))
                            .unwrap();
                    }
                }
            }

            size.width = width;
            size.height = height;

            let size = *RwLockWriteGuard::downgrade(size);

            send(UiEvent::Window(index, WindowEvent::Resize(size)));

            LRESULT(0)
        }
        UM_ANIM_REQUEST => {
            let freq = f64::from_bits(wparam.0 as u64);

            window
                .requested_refresh_rate
                .store(freq as _, Ordering::Release);

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
            let mouse_coords = mouse_coords(lparam);

            {
                let mut pointer_location = window.pointer_location.lock();

                if pointer_location.is_none() {
                    *pointer_location = Some(mouse_coords);

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
            }

            send(UiEvent::Input(index, Input::PointerMove(mouse_coords)));

            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            let _ = window.pointer_location.lock().take();
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
