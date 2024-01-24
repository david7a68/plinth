use std::{
    cell::Cell,
    ptr::addr_of,
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
    limits::MAX_TITLE_LENGTH,
    window::{Axis, ButtonState, Input, MouseButton, WindowEvent, WindowPoint, WindowSize},
    WindowSpec,
};

use super::{
    application::AppMessage,
    ui_thread::UiEvent,
    vsync::{decode_reply_device_update, decode_reply_vsync},
    window::{
        extract_redraw_request, Control, UM_COMPOSITION_RATE, UM_DESTROY_WINDOW, UM_REDRAW_REQUEST,
        UM_VSYNC, WINDOWS_DEFAULT_DPI,
    },
};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

pub fn spawn_event_loop(
    spec: WindowSpec,
    app_sender: Sender<AppMessage>,
    ui_sender: Sender<UiEvent>,
) {
    std::thread::spawn(move || run_event_loop(&spec, app_sender, ui_sender));
}

pub fn run_event_loop(
    spec: &WindowSpec,
    app_sender: Sender<AppMessage>,
    ui_sender: Sender<UiEvent>,
) {
    app_sender.send(AppMessage::WindowCreate).unwrap();

    let wndclass = ensure_wndclass_registered();
    let hwnd = create_window(wndclass, spec);

    ui_sender.send(UiEvent::New(hwnd)).unwrap();

    let mut event_loop = EventLoop::new(ui_sender);

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, addr_of!(event_loop) as *const _ as _);
    }

    if spec.visible {
        unsafe { ShowWindow(hwnd, SW_SHOW) };
    }

    event_loop.run();

    app_sender.send(AppMessage::WindowDestroy).unwrap();
}

struct EventLoop {
    ui_sender: Sender<UiEvent>,
    size: Cell<(u16, u16)>,
    pointer_in_window: Cell<bool>,
    is_in_size_move: Cell<bool>,
    is_resizing: Cell<bool>,
}

impl EventLoop {
    fn new(ui_sender: Sender<UiEvent>) -> Self {
        Self {
            ui_sender,
            size: Cell::new((0, 0)),
            pointer_in_window: Cell::new(false),
            is_in_size_move: Cell::new(false),
            is_resizing: Cell::new(false),
        }
    }

    fn run(&mut self) {
        while self.tick() {}
    }

    fn tick(&mut self) -> bool {
        let mut msg = MSG::default();

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
            0 => false,
            1 => unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
                true
            },
            _ => unreachable!(),
        }
    }
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

fn mouse_coords(lparam: LPARAM) -> WindowPoint {
    let x = (lparam.0 & 0xffff) as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as i16;
    WindowPoint { x, y }
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const EventLoop;

    // :udata_index
    if state.is_null() {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    } else {
        let state = &*state;
        wndproc(state, hwnd, msg, wparam, lparam)
    }
}

#[tracing::instrument(skip(state, hwnd, msg, wparam, lparam))]
fn wndproc(state: &EventLoop, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let send = |event| state.ui_sender.send(event).unwrap();

    match msg {
        WM_CLOSE => {
            send(UiEvent::Window(WindowEvent::CloseRequest));
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            LRESULT(0)
        }
        WM_DESTROY => {
            send(UiEvent::Quit);
            unsafe { PostQuitMessage(0) };
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };

            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            send(UiEvent::Window(WindowEvent::Visible(wparam.0 != 0)));
            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            state.is_in_size_move.set(true);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            state.is_in_size_move.set(false);
            send(UiEvent::Window(WindowEvent::EndResize));
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let (width, height) = unsafe {
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect).unwrap();

                ((rect.right - rect.left) as _, (rect.bottom - rect.top) as _)
            };

            let current_size = state.size.get();
            let is_resizing = width != current_size.0 || height != current_size.1;

            if is_resizing {
                if state.is_in_size_move.get() && !state.is_resizing.get() {
                    send(UiEvent::Window(WindowEvent::BeginResize));
                    state.is_resizing.set(true);
                }

                state.size.set((width, height));

                send(UiEvent::Window(WindowEvent::Resize(WindowSize {
                    width,
                    height,
                    dpi: WINDOWS_DEFAULT_DPI,
                })));
            }

            LRESULT(0)
        }
        UM_REDRAW_REQUEST => {
            let request = extract_redraw_request(wparam, lparam);
            send(UiEvent::ControlEvent(Control::Redraw(request)));
            LRESULT(0)
        }
        UM_VSYNC => {
            let (frame_id, rate) = decode_reply_vsync(wparam, lparam);
            send(UiEvent::ControlEvent(Control::VSync(frame_id, rate)));
            LRESULT(0)
        }
        UM_COMPOSITION_RATE => {
            let (frame_id, rate) = decode_reply_device_update(wparam, lparam);
            send(UiEvent::ControlEvent(Control::VSyncRateChanged(
                frame_id, rate,
            )));
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            send(UiEvent::ControlEvent(Control::OsRepaint));

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
            send(UiEvent::Input(Input::PointerMove(mouse_coords)));
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            send(UiEvent::Input(Input::PointerLeave));
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            send(UiEvent::Input(Input::Scroll(Axis::Y, delta)));
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            send(UiEvent::Input(Input::Scroll(Axis::X, delta)));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Left,
                ButtonState::Pressed,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Left,
                ButtonState::Released,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Right,
                ButtonState::Pressed,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Right,
                ButtonState::Released,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Middle,
                ButtonState::Pressed,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            send(UiEvent::Input(Input::MouseButton(
                MouseButton::Middle,
                ButtonState::Released,
                mouse_coords(lparam),
            )));
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
