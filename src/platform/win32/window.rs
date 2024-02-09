use std::{
    cell::{Cell, RefCell},
    ptr::addr_of,
    sync::Arc,
};

use parking_lot::RwLock;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, EndPaint, RedrawWindow, HBRUSH, PAINTSTRUCT, RDW_INTERNALPAINT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::WM_MOUSELEAVE,
            Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
            WindowsAndMessaging::{
                AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW,
                PostMessageW, PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow,
                TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON,
                HMENU, IDC_ARROW, MSG, PM_NOREMOVE, SW_HIDE, SW_SHOW, WM_APP, WM_CLOSE, WM_DESTROY,
                WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
                WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
                WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED,
                WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use crate::{
    application::AppContext,
    frame::{FrameId, FramesPerSecond, RedrawRequest, RefreshRate},
    geometry::{Point, Scale, Size},
    limits::MAX_WINDOW_TITLE_LENGTH,
    platform::{
        win32::{application::AppMessage, VSyncRequest},
        PlatformEventHandler,
    },
    window::Window,
    Axis, ButtonState, EventHandler, LogicalPixel, MouseButton, PhysicalPixel, WindowSpec,
};

use super::{
    vsync::{decode_reply_device_update, decode_reply_vsync},
    AppContextImpl,
};

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

pub(super) const UM_DESTROY_WINDOW: u32 = WM_APP;
pub(super) const UM_REDRAW_REQUEST: u32 = WM_APP + 1;

pub(super) const UM_VSYNC: u32 = WM_APP + 2;
pub(super) const UM_COMPOSITION_RATE: u32 = WM_APP + 3;

#[derive(Debug, Default)]
pub struct SharedWindowState {
    pub size: Size<u16, PhysicalPixel>,
    pub scale: Scale<f32, PhysicalPixel, LogicalPixel>,

    pub is_visible: bool,
    pub is_resizing: bool,

    pub pointer_location: Option<Point<i16, Window>>,

    pub composition_rate: FramesPerSecond,
    pub actual_refresh_rate: FramesPerSecond,
    pub requested_refresh_rate: FramesPerSecond,
}

pub struct WindowImpl {
    pub hwnd: HWND,
    pub state: Arc<RwLock<SharedWindowState>>,
    pub context: AppContext,
}

impl WindowImpl {
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

    pub fn size(&self) -> Size<u16, PhysicalPixel> {
        let size = self.state.read().size;
        Size::new(size.width, size.height)
    }

    pub fn scale(&self) -> Scale<f32, PhysicalPixel, LogicalPixel> {
        self.state.read().scale
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Option<Point<i16, PhysicalPixel>> {
        let p = self.state.read().pointer_location?;
        Some(Point::new(p.x, p.y))
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

struct EventLoop {
    app: *const AppContextImpl,
    interposer: Cell<*const RefCell<dyn PlatformEventHandler>>,
    size: Cell<(u16, u16)>,
    pointer_in_window: Cell<bool>,
    is_in_size_move: Cell<bool>,
    is_resizing: Cell<bool>,
}

pub fn spawn_window_thread<W, I, C, F>(
    app: AppContextImpl,
    spec: WindowSpec,
    user_constructor: F,
    interposer_constructor: C,
) where
    W: EventHandler,
    I: PlatformEventHandler,
    F: FnOnce(Window) -> W + Send + 'static,
    C: FnOnce(AppContextImpl, W, HWND) -> I + Send + 'static,
{
    std::thread::spawn(move || {
        app.sender.send(AppMessage::WindowCreate).unwrap();

        let wndclass = ensure_wndclass_registered();
        let hwnd = create_window(wndclass, &spec);

        let window = Window::new(WindowImpl {
            hwnd,
            state: Arc::new(RwLock::new(SharedWindowState::default())),
            context: AppContext { inner: app.clone() },
        });

        let user_handler = user_constructor(window);
        let interposer = RefCell::new(interposer_constructor(app.clone(), user_handler, hwnd));

        let event_loop = EventLoop {
            app: addr_of!(app),
            interposer: Cell::new(addr_of!(interposer)),
            size: Cell::new((0, 0)),
            pointer_in_window: Cell::new(false),
            is_in_size_move: Cell::new(false),
            is_resizing: Cell::new(false),
        };

        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, addr_of!(event_loop) as _);
        }

        if spec.visible {
            unsafe { ShowWindow(hwnd, SW_SHOW) };
        }

        let mut msg = MSG::default();

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
        }

        app.sender.send(AppMessage::WindowDestroy).unwrap();
    });
}

fn ensure_wndclass_registered() -> PCWSTR {
    use std::sync::OnceLock;

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
    use arrayvec::ArrayVec;

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

#[allow(clippy::too_many_lines)]
fn wndproc(state: &EventLoop, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    #[inline]
    fn mouse_coords(lparam: LPARAM) -> Point<i16, PhysicalPixel> {
        let x = (lparam.0 & 0xffff) as i16;
        let y = ((lparam.0 >> 16) & 0xffff) as i16;
        (x, y).into()
    }

    let handler = || unsafe { &*state.interposer.get() }.borrow_mut();

    match msg {
        WM_CLOSE => {
            handler().on_close_request();
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };

            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            handler().on_visible(wparam.0 != 0);
            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            state.is_in_size_move.set(true);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            if state.is_resizing.get() {
                state.is_resizing.set(false);
                state.is_in_size_move.set(false);
                handler().on_end_resize();
            }
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
                    handler().on_begin_resize();
                    state.is_resizing.set(true);
                }

                state.size.set((width, height));

                // todo: handle dpi change -dz
                handler().on_resize(Size::new(width, height), Scale::new(1.0, 1.0));
            }

            LRESULT(0)
        }
        UM_REDRAW_REQUEST => {
            let request = extract_redraw_request(wparam, lparam);
            let send = |r| unsafe { &*state.app }.vsync_sender.send(r).unwrap();

            match request {
                RedrawRequest::Idle => send(VSyncRequest::Idle(hwnd)),
                RedrawRequest::Once => unsafe {
                    RedrawWindow(hwnd, None, None, RDW_INTERNALPAINT);
                },
                RedrawRequest::AtFrame(frame_id) => {
                    send(VSyncRequest::AtFrame(hwnd, frame_id));
                }
                RedrawRequest::AtFrameRate(rate) => {
                    send(VSyncRequest::AtFrameRate(hwnd, rate));
                }
            }
            LRESULT(0)
        }
        UM_VSYNC => {
            let (frame_id, rate) = decode_reply_vsync(wparam, lparam);
            handler().on_vsync(frame_id, rate);

            unsafe { RedrawWindow(hwnd, None, None, RDW_INTERNALPAINT) };

            LRESULT(0)
        }
        UM_COMPOSITION_RATE => {
            let (frame_id, rate) = decode_reply_device_update(wparam, lparam);
            handler().on_composition_rate_change(frame_id, rate);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            handler().on_os_repaint();
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
            handler().on_pointer_move(mouse_coords);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            handler().on_pointer_leave();
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            #[allow(clippy::cast_possible_wrap)]
            let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;
            handler().on_scroll(Axis::Y, delta);
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            #[allow(clippy::cast_possible_wrap)]
            let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;
            handler().on_scroll(Axis::X, delta);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            handler().on_mouse_button(
                MouseButton::Left,
                ButtonState::Pressed,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            handler().on_mouse_button(
                MouseButton::Left,
                ButtonState::Released,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            handler().on_mouse_button(
                MouseButton::Right,
                ButtonState::Pressed,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            handler().on_mouse_button(
                MouseButton::Right,
                ButtonState::Released,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            handler().on_mouse_button(
                MouseButton::Middle,
                ButtonState::Pressed,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            handler().on_mouse_button(
                MouseButton::Middle,
                ButtonState::Released,
                mouse_coords(lparam),
            );
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
