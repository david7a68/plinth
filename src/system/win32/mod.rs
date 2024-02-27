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
const UM_DEFER_DESTROY: u32 = WM_APP + 1;
const UM_DEFER_SHOW: u32 = WM_APP + 2;
/// Message used to request a repaint. This is used instead of directly calling
/// `InvalidateRect` so as to consolidate repaint logic to the event loop. This
/// is safe to do since the event loop will not generate WM_PAINT events until
/// the message queue is empty.
///
/// This is slightly less efficient since we need to round-trip into the message
/// queue, but the simplicity was deemed worth it.
const UM_DEFER_PAINT: u32 = WM_APP + 3;

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
        unsafe { PostMessageW(self.hwnd, UM_DEFER_DESTROY, None, None) }.unwrap();
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
        unsafe { PostMessageW(self.hwnd, UM_DEFER_PAINT, None, None) }.unwrap();
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

        let mut opt = Some(constructor);
        let wrap_ctor = RefCell::new(move |window: &api::Window<()>| opt.take().unwrap()(window));

        let create_struct = RefCell::new(CreateStruct {
            wndproc_state: self.opaque_state,
            constructor: &wrap_ctor,
            error: Ok(()),
            title: Some(attributes.title),
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
        api::ActiveEventLoop {
            event_loop: ActiveEventLoop {
                wndclass: self.wndclass,
                opaque_state: self as *const WndProcState<_, _> as *const (),
                _phantom: PhantomData::<*const WindowData>,
            },
        }
    }

    fn get_context(&self, hwnd: HWND) -> Option<HandlerContext<WindowData, H>> {
        let index = self.hwnds.iter().position(|cell| cell.get() == hwnd)?;
        Some(self.get_context_by_index(index))
    }

    fn get_context_by_index(&self, index: usize) -> HandlerContext<WindowData, H> {
        HandlerContext {
            hwnd: &self.hwnds[index],
            data: &self.window_data[index],
            state: &self.window_states[index],
            event_handler: &self.event_handler,
            event_loop: self.as_active_event_loop(),
        }
    }
}

struct CreateStruct<WindowData> {
    wndproc_state: *const (),
    #[allow(clippy::type_complexity)]
    constructor: *const RefCell<dyn FnMut(&api::Window<()>) -> WindowData>,
    /// Place to stash any errors that may occur during window creation.
    error: Result<(), api::WindowError>,
    title: Option<Cow<'static, str>>,
    min_size: PhysicalSize,
    max_size: PhysicalSize,
    is_visible: bool,
    is_resizable: bool,
}

struct HandlerContext<'a, WindowData, H: EventHandler<WindowData>> {
    hwnd: &'a Cell<HWND>,
    data: &'a RefCell<MaybeUninit<WindowData>>,
    state: &'a RefCell<MaybeUninit<WindowState>>,
    event_handler: &'a RefCell<H>,
    event_loop: api::ActiveEventLoop<WindowData>,
}

impl<'a, WindowData, H: EventHandler<WindowData>> HandlerContext<'a, WindowData, H> {
    fn init(&mut self, hwnd: HWND, create_struct: &mut CreateStruct<WindowData>) {
        self.hwnd.set(hwnd);

        unsafe { SetLastError(WIN32_ERROR(0)) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.wndproc_state as _) };
        unsafe { GetLastError() }.expect("SetWindowLongPtrW(GWLP_USERDATA) failed.");

        self.state.borrow_mut().write({
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            assert!(dpi > 0, "GetDpiForWindow failed.");

            let scale = dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32;

            let mut default_rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut default_rect) }.expect("GetClientRect failed.");

            WindowState {
                title: unsafe { create_struct.title.take().unwrap_unchecked() },
                size: PhysicalSize::new(
                    (default_rect.right - default_rect.left) as _,
                    (default_rect.bottom - default_rect.top) as _,
                ),
                max_size: PhysicalSize::new(
                    i16::try_from((create_struct.max_size.width as f32 * scale) as u32)
                        .unwrap_or(i16::MAX),
                    i16::try_from((create_struct.max_size.height as f32 * scale) as u32)
                        .unwrap_or(i16::MAX),
                ),
                min_size: PhysicalSize::new(
                    i16::try_from((create_struct.min_size.width as f32 * scale) as u32).unwrap(),
                    i16::try_from((create_struct.min_size.height as f32 * scale) as u32).unwrap(),
                ),
                position: PhysicalPosition::new(default_rect.left as _, default_rect.top as _),
                dpi: u16::try_from(dpi).unwrap(),
                has_focus: false,
                is_visible: create_struct.is_visible,
                has_pointer: false,
                is_resizable: create_struct.is_resizable,
                is_resizing: false,
                in_drag_resize: false,
                paint_reason: None,
            }
        });

        self.data.borrow_mut().write({
            let state = self.state.borrow();
            let window = api::Window {
                window: Window {
                    hwnd,
                    state: unsafe { state.assume_init_ref() },
                    data: &mut (),
                    _phantom: PhantomData,
                },
            };

            unsafe { (*create_struct.constructor).borrow_mut()(&window) }
        });
    }

    fn wake(&mut self) {
        self.event(|handler, event_loop, window| handler.wake_requested(event_loop, window));
    }

    fn close(&mut self) {
        self.event(|handler, event_loop, window| handler.close_requested(event_loop, window));
    }

    fn destroy_defer(&mut self) {
        unsafe { DestroyWindow(self.hwnd.get()) }.unwrap();
    }

    fn destroy(&mut self) {
        unsafe { SetWindowLongPtrW(self.hwnd.get(), GWLP_USERDATA, 0) };

        self.hwnd.set(HWND::default());

        // SAFETY: Clearing the hwnd marks the data as uninitialized. It will
        // not be read until it is reinitialized for a new window.
        let window_data = unsafe { self.data.borrow_mut().assume_init_read() };

        self.event_handler
            .borrow_mut()
            .destroyed(&self.event_loop, window_data);
    }

    fn show_defer(&mut self, show: SHOW_WINDOW_CMD) {
        unsafe { ShowWindow(self.hwnd.get(), show) };
    }

    fn show(&mut self, is_visible: bool) {
        self.with_state(|window| window.is_visible = is_visible);

        if is_visible {
            self.event(|handler, event_loop, window| handler.shown(event_loop, window));
        } else {
            self.event(|handler, event_loop, window| handler.hidden(event_loop, window));
        }
    }

    fn enter_size_move(&mut self) {
        self.with_state(|window| window.is_resizing = true);
    }

    fn exit_size_move(&mut self) {
        let resize_ended: bool = self.with_state(|window| {
            window.in_drag_resize = false;
            std::mem::take(&mut window.is_resizing)
        });

        if resize_ended {
            self.event(|handler, event_loop, window| handler.drag_resize_ended(event_loop, window));
        }
    }

    fn dpi_changed(&mut self, dpi: u16, rect: &RECT) {
        let size = PhysicalSize::new(
            i16::try_from(rect.right - rect.left).unwrap(),
            i16::try_from(rect.bottom - rect.top).unwrap(),
        );

        self.with_state(|window| {
            window.dpi = dpi;
            window.size = size; // update size to suppress resize event in WM_WINDOWPOSCHANGED
            window.paint_reason = Some(PaintReason::Commanded);
        });

        unsafe {
            SetWindowPos(
                self.hwnd.get(),
                None,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                SET_WINDOW_POS_FLAGS::default(),
            )
        }
        .expect("SetWindowPos failed.");

        let scale = DpiScale::from(dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32);

        self.event(|handler, event_loop, window| {
            handler.dpi_changed(event_loop, window, scale, size)
        });
    }

    fn get_min_max_info(&mut self, mmi: &mut MINMAXINFO) {
        let (min, max, dpi) =
            self.with_state(|window| (window.min_size, window.max_size, window.dpi as u32));

        let os_min_x = unsafe { GetSystemMetricsForDpi(SM_CXMINTRACK, dpi) };
        let os_min_y = unsafe { GetSystemMetricsForDpi(SM_CYMINTRACK, dpi) };
        let os_max_x = unsafe { GetSystemMetricsForDpi(SM_CXMAXTRACK, dpi) };
        let os_max_y = unsafe { GetSystemMetricsForDpi(SM_CYMAXTRACK, dpi) };

        mmi.ptMinTrackSize.x = i32::from(min.width).max(os_min_x);
        mmi.ptMinTrackSize.y = i32::from(min.height).max(os_min_y);
        mmi.ptMaxTrackSize.x = i32::from(max.width).min(os_max_x);
        mmi.ptMaxTrackSize.y = i32::from(max.height).min(os_max_y);
    }

    fn pos_changed(&mut self, pos: &WINDOWPOS) {
        let (x, y, width, height) = (
            i16::try_from(pos.x).unwrap(),
            i16::try_from(pos.y).unwrap(),
            i16::try_from(pos.cx).unwrap(),
            i16::try_from(pos.cy).unwrap(),
        );

        let (resized, moved) = self.with_state(|window| {
            (
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
                },
            )
        });

        if let Some((start, size)) = resized {
            if start {
                self.event(|handler, event_loop, window| {
                    handler.drag_resize_started(event_loop, window)
                });
            }

            self.event(|handler, event_loop, window| handler.resized(event_loop, window, size));
        }

        if let Some(pos) = moved {
            self.event(|handler, event_loop, window| handler.moved(event_loop, window, pos))
        }
    }

    fn defer_paint(&mut self) {
        self.with_state(|window| {
            window.paint_reason = if let Some(reason) = window.paint_reason {
                Some(reason.max(PaintReason::Requested))
            } else {
                Some(PaintReason::Requested)
            }
        });

        unsafe { InvalidateRect(self.hwnd.get(), None, false) }
            .ok()
            .expect("InvalidateRect failed.");
    }

    fn paint(&mut self) {
        let mut ps = PAINTSTRUCT::default();

        let is_invalid = unsafe { BeginPaint(self.hwnd.get(), &mut ps) }.is_invalid();
        assert!(!is_invalid, "BeginPaint failed.");

        unsafe { EndPaint(self.hwnd.get(), &ps) };

        let reason = self
            .with_state(|window| window.paint_reason.take())
            .unwrap_or(PaintReason::Commanded); // assume no reason means it's from the OS

        self.event(|handler, event_loop, window| handler.needs_repaint(event_loop, window, reason));
    }

    fn mouse_move(&mut self, position: PhysicalPosition) {
        let entered = self.with_state(|window| !std::mem::replace(&mut window.has_pointer, true));

        if entered {
            self.event(|handler, event_loop, window| {
                handler.pointer_entered(event_loop, window, position)
            });

            unsafe {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: self.hwnd.get(),
                    dwHoverTime: 0,
                })
            }
            .unwrap()
        }

        self.event(|handler, event_loop, window| {
            handler.pointer_moved(event_loop, window, position)
        });
    }

    fn mouse_leave(&mut self) {
        self.with_state(|window| window.has_pointer = false);
        self.event(|handler, event_loop, window| handler.pointer_left(event_loop, window));
    }

    fn wm_mouse_wheel(&mut self, axis: ScrollAxis, delta: f32, mods: ModifierKeys) {
        self.event(|handler, event_loop, window| {
            handler.mouse_scrolled(event_loop, window, delta, axis, mods)
        });
    }

    fn mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        position: PhysicalPosition,
        mods: ModifierKeys,
    ) {
        self.event(|handler, event_loop, window| {
            handler.mouse_button(event_loop, window, button, state, position, mods)
        });
    }

    #[inline]
    fn with_state<T>(&mut self, f: impl FnOnce(&mut WindowState) -> T) -> T {
        assert_ne!(self.hwnd.get(), HWND::default(), "Window not initialized.");

        let mut state = self.state.borrow_mut();
        let state = unsafe { state.assume_init_mut() };
        f(state)
    }

    #[inline]
    fn event(
        &mut self,
        f: impl FnOnce(&mut H, &api::ActiveEventLoop<WindowData>, &mut api::Window<WindowData>),
    ) {
        assert_ne!(self.hwnd.get(), HWND::default(), "Window not initialized.");

        let (state, mut data) = (self.state.borrow(), self.data.borrow_mut());
        let mut handler = self.event_handler.borrow_mut();
        let mut window = api::Window {
            window: Window {
                hwnd: self.hwnd.get(),
                state: unsafe { state.assume_init_ref() },
                data: unsafe { data.assume_init_mut() },
                _phantom: PhantomData,
            },
        };

        f(&mut *handler, &self.event_loop, &mut window);
    }
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
    fn mouse_coords(lparam: LPARAM) -> PhysicalPosition {
        let x = (lparam.0 & 0xffff) as i16;
        let y = ((lparam.0 >> 16) & 0xffff) as i16;
        (x, y).into()
    }

    fn mouse_modifiers(wparam: WPARAM) -> ModifierKeys {
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

    if msg == WM_CREATE {
        // This block MUST NOT borrow the regular `WndProcState::event_handler`
        // because windows can be created while inside the event handler.
        // Borrowing the event handler while handling WM_CREATE will panic,
        // since the event handler had to have been borrowed in order for
        //  `ActiveEventLoop::create_window` to be called in the first place.
        //
        // -dz 2024-02-25

        let mut cs = {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            let cs = &*(cs.lpCreateParams as *const RefCell<CreateStruct<WindowData>>);
            cs.borrow_mut()
        };

        let state = &*(cs.wndproc_state as *const WndProcState<WindowData, H>);

        let slot_index = state.hwnds.iter().position(|h| h.get() == HWND(0));
        let Some(slot_index) = slot_index else {
            cs.error = Err(api::WindowError::TooManyWindows);
            return LRESULT(-1);
        };

        state.get_context_by_index(slot_index).init(hwnd, &mut cs);

        LRESULT(0)
    } else {
        let state = {
            let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const ();

            if state.is_null() {
                return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
            }

            unsafe { &*(state as *const WndProcState<WindowData, H>) }
        };

        let Some(mut context) = state.get_context(hwnd) else {
            return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        };

        match msg {
            UM_WAKE => context.wake(),
            WM_CLOSE => context.close(),
            UM_DEFER_DESTROY => context.destroy_defer(),
            WM_DESTROY => {
                context.destroy();

                let mut count = 0;
                for h in &state.hwnds {
                    count += (h.get() == HWND::default()) as u32; // branchless count
                }

                if count == 0 {
                    unsafe { PostQuitMessage(0) };
                }
            }
            UM_DEFER_SHOW => context.show_defer(from_defer_show(lparam)),
            WM_SHOWWINDOW => context.show(wparam.0 != 0),
            WM_ENTERSIZEMOVE => context.enter_size_move(),
            WM_EXITSIZEMOVE => context.exit_size_move(),
            WM_DPICHANGED => {
                let rect = unsafe { &*(lparam.0 as *const RECT) };
                context.dpi_changed(u16::try_from(wparam.0).unwrap(), rect);
            }
            WM_GETMINMAXINFO => {
                context.get_min_max_info(unsafe { &mut *(lparam.0 as *mut MINMAXINFO) })
            }
            WM_WINDOWPOSCHANGED => context.pos_changed(unsafe { &*(lparam.0 as *const WINDOWPOS) }),
            UM_DEFER_PAINT => context.defer_paint(),
            WM_PAINT => context.paint(),
            WM_MOUSEMOVE => context.mouse_move(mouse_coords(lparam)),
            WM_MOUSELEAVE => context.mouse_leave(),
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                const AXES: [ScrollAxis; 2] = [ScrollAxis::Vertical, ScrollAxis::Horizontal];

                #[allow(clippy::cast_possible_wrap)]
                let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;
                let axis = AXES[(msg == WM_MOUSEHWHEEL) as usize];
                let mods = mouse_modifiers(wparam);
                context.wm_mouse_wheel(axis, delta, mods);
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

                let (button, state) = STATES[(msg - WM_LBUTTONDOWN) as usize];
                let point = mouse_coords(lparam);
                let mods = mouse_modifiers(wparam);
                context.mouse_button(button, state, point, mods);
            }
            // TODO: keyboard input
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        };

        LRESULT(0)
    }
}
