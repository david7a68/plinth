use std::{cell::Cell, ptr::addr_of, sync::Arc};

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
    limits::MAX_TITLE_LENGTH,
    math::{Point, Scale, Size},
    platform::win32::application::AppMessage,
    window::{Window, WindowPoint, WindowSize},
    Axis, ButtonState, Input, MouseButton, WindowEvent, WindowEventHandler, WindowSpec,
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

pub(super) const WINDOWS_DEFAULT_DPI: u16 = 96;

#[derive(Clone, Copy, Debug)]
pub enum Control {
    /// The client applicaton has requested that the window be repainted.
    Redraw(RedrawRequest),
    /// The OS has requested that the window be repainted.
    OsRepaint,
    VSync(FrameId, Option<FramesPerSecond>),
    VSyncRateChanged(FrameId, FramesPerSecond),
}

#[derive(Debug, Default)]
pub struct SharedWindowState {
    pub size: WindowSize,

    pub is_visible: bool,
    pub is_resizing: bool,

    pub pointer_location: Option<WindowPoint>,

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
    pub(super) fn new(
        hwnd: HWND,
        state: Arc<RwLock<SharedWindowState>>,
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

    pub fn size(&self) -> Size<Window> {
        let size = self.state.read().size;
        Size::new(size.width as f32, size.height as f32)
    }

    pub fn scale(&self) -> Scale<Window, Window> {
        let dpi = self.state.read().size.dpi;

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
        let p = self.state.read().pointer_location?;
        Some(Point::new(p.x as f32, p.y as f32))
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

pub trait Win32WindowEventInterposer: 'static {
    fn on_event(&self, event: WindowEvent);
    fn on_input(&self, input: Input);
    fn on_os_paint(&self);
    fn on_vsync(&self, frame_id: FrameId, rate: Option<FramesPerSecond>);
    fn on_composition_rate(&self, frame_id: FrameId, rate: FramesPerSecond);
    fn on_redraw_request(&self, request: RedrawRequest);
}

struct EventLoop {
    interposer: Cell<*const dyn Win32WindowEventInterposer>,
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
    W: WindowEventHandler,
    I: Win32WindowEventInterposer,
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
        let interposer = interposer_constructor(app.clone(), user_handler, hwnd);

        let event_loop = EventLoop {
            interposer: Cell::new(addr_of!(interposer)),
            size: Cell::new((0, 0)),
            pointer_in_window: Cell::new(false),
            is_in_size_move: Cell::new(false),
            is_resizing: Cell::new(false),
        };

        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, addr_of!(event_loop) as *const _ as _);
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
    fn mouse_coords(lparam: LPARAM) -> WindowPoint {
        let x = (lparam.0 & 0xffff) as i16;
        let y = ((lparam.0 >> 16) & 0xffff) as i16;
        WindowPoint { x, y }
    }

    let interposer = unsafe { &*state.interposer.get() };

    match msg {
        WM_CLOSE => {
            interposer.on_event(WindowEvent::CloseRequest);
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
            interposer.on_event(WindowEvent::Visible(wparam.0 != 0));
            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            state.is_in_size_move.set(true);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            state.is_in_size_move.set(false);
            interposer.on_event(WindowEvent::EndResize);
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
                    interposer.on_event(WindowEvent::BeginResize);

                    state.is_resizing.set(true);
                }

                state.size.set((width, height));

                interposer.on_event(WindowEvent::Resize(WindowSize {
                    width,
                    height,
                    dpi: WINDOWS_DEFAULT_DPI,
                }));
            }

            LRESULT(0)
        }
        UM_REDRAW_REQUEST => {
            let request = extract_redraw_request(wparam, lparam);
            interposer.on_redraw_request(request);
            LRESULT(0)
        }
        UM_VSYNC => {
            let (frame_id, rate) = decode_reply_vsync(wparam, lparam);
            interposer.on_vsync(frame_id, rate);

            unsafe { RedrawWindow(hwnd, None, None, RDW_INTERNALPAINT) };

            LRESULT(0)
        }
        UM_COMPOSITION_RATE => {
            let (frame_id, rate) = decode_reply_device_update(wparam, lparam);
            interposer.on_composition_rate(frame_id, rate);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            interposer.on_os_paint();
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
            interposer.on_input(Input::PointerMove(mouse_coords));
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            interposer.on_input(Input::PointerLeave);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            interposer.on_input(Input::Scroll(Axis::Y, delta));
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            interposer.on_input(Input::Scroll(Axis::X, delta));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Left,
                ButtonState::Pressed,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Left,
                ButtonState::Released,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Right,
                ButtonState::Pressed,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Right,
                ButtonState::Released,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Middle,
                ButtonState::Pressed,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            interposer.on_input(Input::MouseButton(
                MouseButton::Middle,
                ButtonState::Released,
                mouse_coords(lparam),
            ));
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
