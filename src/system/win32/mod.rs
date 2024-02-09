pub mod time;

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::MaybeUninit,
};

use arrayvec::ArrayVec;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{
            GetLastError, SetLastError, HWND, LPARAM, LRESULT, RECT, WIN32_ERROR, WPARAM,
        },
        Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::WM_MOUSELEAVE,
            HiDpi::{
                GetDpiForWindow, GetSystemMetricsForDpi, SetProcessDpiAwareness,
                SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                PROCESS_PER_MONITOR_DPI_AWARE,
            },
            Input::KeyboardAndMouse::{
                GetKeyState, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_MENU,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect,
                GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW, PostMessageW,
                PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
                TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
                GWLP_USERDATA, HICON, IDC_ARROW, MINMAXINFO, MSG, PM_NOREMOVE,
                SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD, SM_CXMAXTRACK, SM_CXMINTRACK, SM_CYMAXTRACK,
                SM_CYMINTRACK, SW_HIDE, SW_NORMAL, USER_DEFAULT_SCREEN_DPI, WINDOWPOS, WM_APP,
                WM_CLOSE, WM_CREATE, WM_DESTROY, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
                WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_MBUTTONDBLCLK, WM_MOUSEHWHEEL, WM_MOUSEMOVE,
                WM_MOUSEWHEEL, WM_PAINT, WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED, WNDCLASSEXW,
                WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use crate::{
    frame::FramesPerSecond,
    geometry::Size,
    limits::{MAX_WINDOWS, MAX_WINDOW_TITLE_LENGTH},
    system::input::{ButtonState, ModifierKeys, MouseButton, ScrollAxis},
};

use super::{
    event_loop::EventHandler,
    window::{
        DpiScale, PaintReason, PhysicalPosition, PhysicalSize, RefreshRateRequest, WindowAttributes,
    },
};

mod api {
    pub use crate::system::event_loop::{ActiveEventLoop, EventLoopError};
    pub use crate::system::window::{Window, WindowError, WindowWaker};
}

const UM_WAKE: u32 = WM_APP;
const UM_DESTROY: u32 = WM_APP + 1;
const UM_DEFER_SHOW: u32 = WM_APP + 2;

/// Message used to request a repaint. This is used instead of directly calling
/// `InvalidateRect` so as to consolidate repaint logic to the event loop. This
/// is safe to do since the event loop will not generate WM_PAINT events until
/// the message queue is empty.
///
/// This is slightly less efficient since we need to round-trip into the message
/// queue, but the simplicity was deemed worth it. -dz (2024-02-24)
const UM_REPAINT: u32 = WM_APP + 3;

const WND_CLASS_NAME: PCWSTR = w!("plinth_wc");

#[derive(Clone, Debug, thiserror::Error)]
pub enum WindowError {
    #[error("Window creation failed: {0:?}")]
    CreateFailed(windows::core::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum EventLoopError {
    #[error("[likely bug] An error occurred while registering the window class: {0}")]
    RegisterClassFailed(windows::core::Error),

    #[error("An OS error has occurred. This is likely a bug. {0}")]
    Internal(windows::core::Error),
}

pub struct Window<'a, Data> {
    hwnd: HWND,
    state: &'a WindowState,
    data: &'a mut Data,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<Data> Window<'_, Data> {
    pub fn waker(&self) -> api::WindowWaker {
        api::WindowWaker {
            waker: WindowWaker { target: self.hwnd },
        }
    }

    pub fn destroy(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DESTROY, None, None) }.unwrap();
    }

    pub fn data(&self) -> &Data {
        self.data
    }

    pub fn data_mut(&mut self) -> &mut Data {
        self.data
    }

    pub fn title(&self) -> &str {
        &self.state.title
    }

    #[allow(unused_variables)]
    pub fn set_title(&mut self, title: &str) {
        todo!()
    }

    pub fn size(&self) -> PhysicalSize {
        self.state.size
    }

    #[allow(unused_variables)]
    pub fn set_size(&mut self, size: PhysicalSize) {
        todo!()
    }

    pub fn min_size(&self) -> PhysicalSize {
        self.state.min_size
    }

    pub fn set_min_size(&mut self, min_size: PhysicalSize) {
        todo!()
    }

    pub fn max_size(&self) -> PhysicalSize {
        self.state.max_size
    }

    pub fn set_max_size(&mut self, max_size: PhysicalSize) {
        todo!()
    }

    pub fn position(&self) -> PhysicalPosition {
        self.state.position
    }

    #[allow(unused_variables)]
    pub fn set_position(&mut self, position: PhysicalPosition) {
        todo!()
    }

    pub fn is_visible(&self) -> bool {
        self.state.is_visible
    }

    pub fn show(&mut self) {
        post_defer_show(self.hwnd, SW_NORMAL);
    }

    pub fn hide(&mut self) {
        post_defer_show(self.hwnd, SW_HIDE);
    }

    pub fn is_resizable(&self) -> bool {
        self.state.is_resizable
    }

    pub fn dpi_scale(&self) -> DpiScale {
        let scale = self.state.dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32;
        DpiScale::new(scale, scale)
    }

    pub fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    pub fn has_pointer(&self) -> bool {
        self.state.has_pointer
    }

    pub fn frame_rate(&self) -> FramesPerSecond {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn request_refresh_rate(&mut self, rate: RefreshRateRequest, after_next_present: bool) {
        todo!()
    }

    pub fn request_repaint(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_REPAINT, None, None) }.unwrap();
    }
}

pub struct WindowWaker {
    target: HWND,
}

impl WindowWaker {
    pub fn wake(&self) {
        let _ = unsafe { PostMessageW(self.target, UM_WAKE, None, None) };
    }
}

pub struct ActiveEventLoop<WindowData> {
    wndclass: PCWSTR,
    opaque_state: *const (),
    _phantom: PhantomData<*const WindowData>,
}

impl<WindowData> ActiveEventLoop<WindowData> {
    pub fn create_window(
        &self,
        attributes: WindowAttributes,
        constructor: impl FnOnce(&api::Window<()>) -> WindowData + 'static,
    ) -> Result<(), api::WindowError> {
        if attributes.title.as_ref().len() > MAX_WINDOW_TITLE_LENGTH {
            return Err(api::WindowError::TitleTooLong);
        }

        let title = attributes
            .title
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<ArrayVec<_, { MAX_WINDOW_TITLE_LENGTH + 1 }>>();

        let style = WS_OVERLAPPEDWINDOW;
        let style_ex = WS_EX_NOREDIRECTIONBITMAP;

        let min_size = attributes.min_size.unwrap_or_default();
        let max_size = attributes.max_size.unwrap_or(Size::new(i16::MAX, i16::MAX));

        let (width, height) = attributes
            .size
            .map(|s| (s.width as i32, s.height as i32))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let (x, y) = attributes
            .position
            .map(|p| (p.x as i32, p.y as i32))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let mut opt = Some(constructor); // :move-ctor:
        let wrap_ctor = RefCell::new(move |window: &api::Window<()>| opt.take().unwrap()(window)); // :move-ctor:

        let create_struct = RefCell::new(CreateStruct {
            wndproc_state: self.opaque_state,
            constructor: &wrap_ctor,
            error: Ok(()),
            title: Some(attributes.title), // :move-title:
            min_size,
            max_size,
            is_visible: attributes.is_visible,
            is_resizable: attributes.is_resizable,
        });

        let hwnd = unsafe {
            CreateWindowExW(
                style_ex,
                self.wndclass,
                PCWSTR(title.as_ptr()),
                style,
                x,
                y,
                width,
                height,
                None,
                None,
                None,
                Some((&create_struct as *const RefCell<_>).cast()),
            )
        };

        debug_assert!(create_struct.try_borrow().is_ok());

        // SAFETY: This is safe because `CreateWindowExW` returns only after
        // WM_CREATE, which does not persist any references to the `RefCell`
        // once it returns.
        let create_struct = create_struct.into_inner();

        // Return error if the window creation failed within the callback.
        create_struct.error?;

        // Return error if the window creation failed outside the callback.
        // This must happen after checking the returned error since falling
        // within the callback will also fail window creation.
        if hwnd == HWND::default() {
            let err = unsafe { GetLastError() }.unwrap_err();
            Err(WindowError::CreateFailed(err))?;
        }

        post_defer_show(hwnd, SW_NORMAL);

        Ok(())
    }
}

pub struct EventLoop {}

impl EventLoop {
    pub fn new() -> Result<Self, EventLoopError> {
        Ok(Self {})
    }

    pub fn run<WindowData, H: EventHandler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), api::EventLoopError> {
        use windows_version::OsVersion;

        let os_version = OsVersion::current();
        if os_version >= OsVersion::new(10, 0, 0, 1703) {
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
                .map_err(EventLoopError::Internal)?;
        } else if os_version >= OsVersion::new(8, 1, 0, 0) {
            unsafe { SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE) }
                .map_err(EventLoopError::Internal)?;
        } else {
            // Windows 8.0 and earlier are not supported
            Err(api::EventLoopError::UnsupportedOsVersion)?;
        }

        let wndclass = {
            let atom = unsafe {
                RegisterClassExW(&WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(unsafe_wndproc::<WindowData, H>),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: GetModuleHandleW(None).unwrap().into(),
                    hIcon: HICON::default(),
                    hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                    hbrBackground: HBRUSH::default(),
                    lpszMenuName: PCWSTR::null(),
                    lpszClassName: WND_CLASS_NAME,
                    hIconSm: HICON::default(),
                })
            };

            if atom == 0 {
                let err = unsafe { GetLastError() }.unwrap_err();
                Err(EventLoopError::RegisterClassFailed(err))?;
            }

            PCWSTR(atom as usize as *const _)
        };

        let wndproc_state = WndProcState::<WindowData, H> {
            wndclass,
            event_handler: RefCell::new(event_handler),

            hwnds: [(); MAX_WINDOWS].map(|_| Cell::new(HWND::default())),
            window_data: [(); MAX_WINDOWS].map(|_| RefCell::new(MaybeUninit::uninit())),
            window_states: [(); MAX_WINDOWS].map(|_| RefCell::new(MaybeUninit::uninit())),
        };

        let event_loop = wndproc_state.as_active_event_loop();

        wndproc_state.event_handler.borrow_mut().start(&event_loop);

        // Run the event loop until completion.
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

        wndproc_state.event_handler.borrow_mut().stop();

        Ok(())
    }
}

struct WindowState {
    title: Cow<'static, str>,
    size: PhysicalSize,
    min_size: PhysicalSize,
    max_size: PhysicalSize,
    position: PhysicalPosition,
    dpi: u16,
    has_focus: bool,
    is_visible: bool,
    has_pointer: bool,
    is_resizable: bool,
    is_resizing: bool,
    /// Keep this per-window, not per-event-loop because a different window
    /// might get a resize event while this one is still resizing. If that
    /// happens, we don't want the other window to get resize begin/end events.
    in_drag_resize: bool,
    paint_reason: Option<PaintReason>,
}

struct WndProcState<WindowData, H: EventHandler<WindowData>> {
    wndclass: PCWSTR,
    event_handler: RefCell<H>,

    hwnds: [Cell<HWND>; MAX_WINDOWS],
    window_data: [RefCell<MaybeUninit<WindowData>>; MAX_WINDOWS],
    window_states: [RefCell<MaybeUninit<WindowState>>; MAX_WINDOWS],
}

impl<WindowData, H: EventHandler<WindowData>> WndProcState<WindowData, H> {
    fn as_active_event_loop(&self) -> api::ActiveEventLoop<WindowData> {
        let event_loop = ActiveEventLoop {
            wndclass: self.wndclass,
            opaque_state: self as *const WndProcState<_, _> as *const (),
            _phantom: PhantomData::<*const WindowData>,
        };

        api::ActiveEventLoop { event_loop }
    }
}

struct CreateStruct<WindowData> {
    wndproc_state: *const (),
    /// :move-ctor: Wraps a `FnOnce`. Will panic if called more than once.
    #[allow(clippy::type_complexity)]
    constructor: *const RefCell<dyn FnMut(&api::Window<()>) -> WindowData>,
    /// Place to stash any errors that may occur during window creation.
    error: Result<(), api::WindowError>,
    /// :move-title: Used to 'move' the title from `create_window` to `WM_CREATE`.
    title: Option<Cow<'static, str>>,
    min_size: PhysicalSize,
    max_size: PhysicalSize,
    is_visible: bool,
    is_resizable: bool,
}

fn mouse_coords(lparam: LPARAM) -> PhysicalPosition {
    let x = (lparam.0 & 0xffff) as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as i16;
    (x, y).into()
}

fn mouse_modifier_keys(wparam: WPARAM) -> ModifierKeys {
    const MK_CONTROL: usize = 0x0008;
    const MK_SHIFT: usize = 0x0004;

    let mut modifiers = ModifierKeys::empty();

    if wparam.0 & MK_CONTROL != 0 {
        modifiers |= ModifierKeys::CTRL;
    }

    if wparam.0 & MK_SHIFT != 0 {
        modifiers |= ModifierKeys::SHIFT;
    }

    if unsafe { GetKeyState(i32::from(VK_MENU.0)) } < 0 {
        modifiers |= ModifierKeys::ALT;
    }

    modifiers
}

/// Add a message to the queue to show or hide a window. Necessary because
/// `ShowWindow` is synchronous, which can call the wndproc re-entrantly.
fn post_defer_show(hwnd: HWND, show: SHOW_WINDOW_CMD) {
    unsafe { PostMessageW(hwnd, UM_DEFER_SHOW, None, LPARAM(show.0 as _)) }.unwrap();
}

/// Extract the `SHOW_WINDOW_CMD` from the `LPARAM` of a `UM_DEFER_SHOW`
/// message.
fn from_defer_show(lparam: LPARAM) -> SHOW_WINDOW_CMD {
    SHOW_WINDOW_CMD(lparam.0 as i32)
}

unsafe extern "system" fn unsafe_wndproc<WindowData, H: EventHandler<WindowData>>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        // This block MUST NOT borrow the regular `WndProcState::event_handler`
        // because windows can be created while inside the event handler.
        // Borrowing the event handler while handling WM_CREATE will panic,
        // since the event handler had to have been borrowed in order for
        //  `ActiveEventLoop::create_window` to be called in the first place.
        //
        // -dz 2024-02-25

        let csw = &*(lparam.0 as *const CREATESTRUCTW);
        let cs = &*(csw.lpCreateParams as *const RefCell<CreateStruct<WindowData>>);
        let mut cs = cs.borrow_mut();
        let proc_state = &*(cs.wndproc_state as *const WndProcState<WindowData, H>);

        let Some((i, hwnd_slot)) = proc_state
            .hwnds
            .iter()
            .enumerate()
            .find(|(_, hwnd)| hwnd.get() == HWND::default())
        else {
            cs.error = Err(api::WindowError::TooManyWindows);
            return LRESULT(-1);
        };

        hwnd_slot.set(hwnd);

        unsafe {
            SetLastError(WIN32_ERROR(0));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.wndproc_state as _);
            GetLastError().expect("SetWindowLongPtrW(GWLP_USERDATA) failed.");
        };

        let dpi = unsafe { GetDpiForWindow(hwnd) };
        assert!(dpi > 0, "GetDpiForWindow failed.");

        let scale = dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32;

        let mut default_rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut default_rect) }.expect("GetClientRect failed.");

        let state = WindowState {
            title: unsafe { cs.title.take().unwrap_unchecked() }, // :move-title:
            size: PhysicalSize::new(
                (default_rect.right - default_rect.left) as _,
                (default_rect.bottom - default_rect.top) as _,
            ),
            max_size: PhysicalSize::new(
                i16::try_from((cs.max_size.width as f32 * scale) as u32).unwrap_or(i16::MAX),
                i16::try_from((cs.max_size.height as f32 * scale) as u32).unwrap_or(i16::MAX),
            ),
            min_size: PhysicalSize::new(
                i16::try_from((cs.min_size.width as f32 * scale) as u32).unwrap(),
                i16::try_from((cs.min_size.height as f32 * scale) as u32).unwrap(),
            ),
            position: PhysicalPosition::new(default_rect.left as _, default_rect.top as _),
            dpi: u16::try_from(dpi).unwrap(),
            has_focus: false,
            is_visible: cs.is_visible,
            has_pointer: false,
            is_resizable: cs.is_resizable,
            is_resizing: false,
            in_drag_resize: false,
            paint_reason: None,
        };

        let state_slot = &proc_state.window_states[i];

        state_slot.borrow_mut().write(state);

        proc_state.window_data[i].borrow_mut().write({
            let state_slot = state_slot.borrow();

            let window = api::Window {
                window: Window {
                    hwnd,
                    state: unsafe { state_slot.assume_init_ref() },
                    data: &mut (),
                    _phantom: PhantomData,
                },
            };

            (*cs.constructor).borrow_mut()(&window) // :move-ctor:
        });

        LRESULT(0)
    } else {
        let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const ();

        if state.is_null() {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        } else {
            let state = unsafe { &*(state as *const WndProcState<WindowData, H>) };
            let i = state.hwnds.iter().position(|h| h.get() == hwnd);

            // This shouldn't be necessary, but just in case.
            match i {
                Some(i) => wndproc(state, i, hwnd, msg, wparam, lparam),
                None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
    }
}

fn wndproc<WindowData, H: EventHandler<WindowData>>(
    state: &WndProcState<WindowData, H>,
    slot: usize,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let event_loop = state.as_active_event_loop();

    macro_rules! with_state {
        ($expr:expr) => {{
            /// Function used to infer the types of the closure's parameters.
            #[inline(always)]
            fn passthrough<T, F: FnMut(&mut WindowState) -> T>(func: F) -> F {
                func
            }

            let mut window_state = state.window_states[slot].borrow_mut();
            let window_state = unsafe { window_state.assume_init_mut() };

            passthrough($expr)(window_state)
        }};
    }

    macro_rules! event {
        ($expr:expr) => {{
            /// Function used to infer the types of the closure's parameters.
            #[inline(always)]
            fn passthrough<
                WindowData,
                H: EventHandler<WindowData>,
                F: FnMut(&mut H, &api::ActiveEventLoop<WindowData>, &mut api::Window<WindowData>),
            >(
                func: F,
            ) -> F {
                func
            }

            let window_state = state.window_states[slot].borrow();
            let mut window_data = state.window_data[slot].borrow_mut();

            let mut window = api::Window {
                window: Window {
                    hwnd,
                    state: unsafe { window_state.assume_init_ref() },
                    data: unsafe { window_data.assume_init_mut() },
                    _phantom: PhantomData,
                },
            };

            let mut handler = state.event_handler.borrow_mut();
            passthrough::<WindowData, H, _>($expr)(&mut *handler, &event_loop, &mut window);
        }};
    }

    match msg {
        UM_WAKE => {
            event!(|handler, event_loop, window| handler.window_wake_requested(event_loop, window));

            LRESULT(0)
        }
        WM_CLOSE => {
            event!(|handler, event_loop, window| handler.window_close_requested(event_loop, window));

            LRESULT(0)
        }
        UM_DESTROY => {
            unsafe { DestroyWindow(hwnd) }.unwrap();

            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY:
            //
            // The window state is initialized in the WM_CREATE message.
            // WM_DESTROY is the last message received by a window (save for
            // WM_NCDESTROY, which we're not handling).
            let window_data = unsafe { state.window_data[slot].borrow_mut().assume_init_read() };

            state
                .event_handler
                .borrow_mut()
                .window_destroyed(&event_loop, window_data);

            state.hwnds[slot].set(HWND::default());

            let count = state
                .hwnds
                .iter()
                .filter(|h| h.get() != HWND::default())
                .count();

            if count == 0 {
                unsafe { PostQuitMessage(0) };
            }

            LRESULT(0)
        }
        UM_DEFER_SHOW => {
            let show = from_defer_show(lparam);
            unsafe { ShowWindow(hwnd, show) };

            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            let is_visible = wparam.0 != 0;

            with_state!(|window| window.is_visible = is_visible);

            if is_visible {
                let window_state = state.window_states[slot].borrow();
                let mut window_data = state.window_data[slot].borrow_mut();

                let mut window = api::Window {
                    window: Window {
                        hwnd,
                        state: unsafe { window_state.assume_init_ref() },
                        data: unsafe { window_data.assume_init_mut() },
                        _phantom: PhantomData,
                    },
                };

                let mut handler = state.event_handler.borrow_mut();

                handler.window_shown(&event_loop, &mut window);
                // event!(|handler, event_loop, window| handler.window_shown(event_loop, window));
            } else {
                event!(|handler, event_loop, window| handler.window_hidden(event_loop, window));
            }

            LRESULT(0)
        }
        WM_ENTERSIZEMOVE => {
            with_state!(|window| {
                window.is_resizing = true;
            });

            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            let resize_ended = with_state!(|window| {
                window.in_drag_resize = false;
                std::mem::take(&mut window.is_resizing)
            });

            if resize_ended {
                event!(|handler, event_loop, window| handler
                    .window_drag_resize_ended(event_loop, window));
            }

            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = wparam.0 as u16;
            let rect = unsafe { &*(lparam.0 as *const RECT) };
            let size = PhysicalSize::new(
                i16::try_from(rect.right - rect.left).unwrap(),
                i16::try_from(rect.bottom - rect.top).unwrap(),
            );

            with_state!(|window| {
                window.dpi = dpi;
                window.size = size; // update size to suppress resize event in WM_WINDOWPOSCHANGED
                window.paint_reason = Some(PaintReason::Commanded);
            });

            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    SET_WINDOW_POS_FLAGS::default(),
                )
            }
            .unwrap();

            let scale = DpiScale::from(dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32);
            event!(|handler, event_loop, window| handler
                .window_dpi_changed(event_loop, window, scale, size));

            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let minmaxinfo = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };

            let (min, max, dpi) =
                with_state!(|window| { (window.min_size, window.max_size, window.dpi as u32) });

            let os_min_x = unsafe { GetSystemMetricsForDpi(SM_CXMINTRACK, dpi) };
            let os_min_y = unsafe { GetSystemMetricsForDpi(SM_CYMINTRACK, dpi) };
            let os_max_x = unsafe { GetSystemMetricsForDpi(SM_CXMAXTRACK, dpi) };
            let os_max_y = unsafe { GetSystemMetricsForDpi(SM_CYMAXTRACK, dpi) };

            minmaxinfo.ptMinTrackSize.x = i32::from(min.width).max(os_min_x);
            minmaxinfo.ptMinTrackSize.y = i32::from(min.height).max(os_min_y);
            minmaxinfo.ptMaxTrackSize.x = i32::from(max.width).min(os_max_x);
            minmaxinfo.ptMaxTrackSize.y = i32::from(max.height).min(os_max_y);

            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let (x, y, width, height) = unsafe {
                let window_pos = &*(lparam.0 as *const WINDOWPOS);
                (
                    i16::try_from(window_pos.x).unwrap(),
                    i16::try_from(window_pos.y).unwrap(),
                    i16::try_from(window_pos.cx).unwrap(),
                    i16::try_from(window_pos.cy).unwrap(),
                )
            };

            let (resized, moved) = with_state!(|window| (
                {
                    let resized = width != window.size.width || height != window.size.height;
                    let is_start = resized && window.in_drag_resize;

                    window.size = PhysicalSize::new(width, height);
                    window.is_resizing = is_start;
                    window.paint_reason = resized
                        .then_some(PaintReason::Commanded) // override if resized
                        .or(window.paint_reason); // else keep the current reason

                    resized.then_some((is_start, window.size))
                },
                {
                    let moved = x != window.position.x || y != window.position.y;

                    window.position = PhysicalPosition::new(x, y); // doesn't matter if it's the same

                    moved.then_some(window.position)
                }
            ));

            if let Some((start, size)) = resized {
                if start {
                    event!(|handler, event_loop, window| handler
                        .window_drag_resize_started(event_loop, window));
                }

                event!(
                    |handler, event_loop, window| handler.window_resized(event_loop, window, size)
                );
            }

            if let Some(pos) = moved {
                event!(|handler, event_loop, window| handler.window_moved(event_loop, window, pos))
            }

            LRESULT(0)
        }
        UM_REPAINT => {
            with_state!(|window| {
                window.paint_reason = window
                    .paint_reason
                    .map(|reason| reason.max(PaintReason::Requested))
                    .or(Some(PaintReason::Requested));
            });

            unsafe { InvalidateRect(hwnd, None, false) };

            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();

            unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            let reason = with_state!(|window| { window.paint_reason.take() })
                .unwrap_or(PaintReason::Commanded); // assume no reason means it's from the OS

            event!(|handler, event_loop, window| handler
                .window_needs_repaint(event_loop, window, reason));

            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (entered, position) = with_state!(|window| {
                let entered = !window.has_pointer;
                window.has_pointer = true;
                (entered, mouse_coords(lparam))
            });

            if entered {
                event!(|handler, event_loop, window| handler
                    .input_pointer_entered(event_loop, window, position));

                unsafe {
                    TrackMouseEvent(&mut TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    })
                }
                .unwrap()
            }

            event!(|handler, event_loop, window| handler
                .input_pointer_move(event_loop, window, position));

            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            with_state!(|window| window.has_pointer = false);

            event!(|handler, event_loop, window| handler.input_pointer_leave(event_loop, window));

            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            const AXES: [ScrollAxis; 2] = [ScrollAxis::Vertical, ScrollAxis::Horizontal];

            #[allow(clippy::cast_possible_wrap)]
            let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;
            let axis = AXES[(msg == WM_MOUSEHWHEEL) as usize];

            event!(|handler, event_loop, window| handler.input_scroll(
                event_loop,
                window,
                delta,
                axis,
                mouse_modifier_keys(wparam)
            ));

            LRESULT(0)
        }
        WM_LBUTTONDOWN..=WM_MBUTTONDBLCLK => {
            const STATES: [(MouseButton, ButtonState); 9] = [
                (MouseButton::Left, ButtonState::Pressed),
                (MouseButton::Left, ButtonState::Released),
                (MouseButton::Left, ButtonState::DoubleTapped),
                (MouseButton::Right, ButtonState::Pressed),
                (MouseButton::Right, ButtonState::Released),
                (MouseButton::Right, ButtonState::DoubleTapped),
                (MouseButton::Middle, ButtonState::Pressed),
                (MouseButton::Middle, ButtonState::Released),
                (MouseButton::Middle, ButtonState::DoubleTapped),
            ];

            let position = mouse_coords(lparam);
            let modifiers = mouse_modifier_keys(wparam);
            let (button, state) = STATES[(msg - WM_LBUTTONDOWN) as usize];

            event!(|handler, event_loop, window| handler
                .input_mouse_button(event_loop, window, button, state, position, modifiers));

            LRESULT(0)
        }
        // TODO: keyboard input
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
