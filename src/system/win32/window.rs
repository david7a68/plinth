use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::MaybeUninit,
};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{
            GetLastError, SetLastError, HWND, LPARAM, LRESULT, RECT, WIN32_ERROR, WPARAM,
        },
        Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
            WindowsAndMessaging::{
                DestroyWindow, GetClientRect, LoadCursorW, PostMessageW, RegisterClassExW,
                SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA,
                HICON, IDC_ARROW, MINMAXINFO, SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD, SM_CXMAXTRACK,
                SM_CXMINTRACK, SM_CYMAXTRACK, SM_CYMINTRACK, SW_HIDE, SW_NORMAL,
                USER_DEFAULT_SCREEN_DPI, WINDOWPOS, WM_APP, WNDCLASSEXW,
            },
        },
    },
};

use crate::{
    frame::FramesPerSecond,
    system::{
        dpi::{DpiScale, WindowPoint, WindowSize},
        event_loop::EventHandler,
        input::{ButtonState, ModifierKeys, MouseButton, ScrollAxis},
        window::{PaintReason, RefreshRateRequest},
    },
};

use super::api;

pub(crate) const UM_WAKE: u32 = WM_APP;
pub(crate) const UM_DEFER_DESTROY: u32 = WM_APP + 1;
pub(crate) const UM_DEFER_SHOW: u32 = WM_APP + 2;
/// Message used to request a repaint. This is used instead of directly calling
/// `InvalidateRect` so as to consolidate repaint logic to the event loop. This
/// is safe to do since the event loop will not generate WM_PAINT events until
/// the message queue is empty.
///
/// This is slightly less efficient since we need to round-trip into the message
/// queue, but the simplicity was deemed worth it.
pub(crate) const UM_DEFER_PAINT: u32 = WM_APP + 3;

const WND_CLASS_NAME: PCWSTR = w!("plinth_wc");

#[derive(Clone, Debug, thiserror::Error)]
pub enum WindowError {
    #[error("Window creation failed: {0:?}")]
    CreateFailed(windows::core::Error),
}

pub struct WindowWaker {
    target: HWND,
}

impl WindowWaker {
    pub fn wake(&self) {
        let _ = unsafe { PostMessageW(self.target, UM_WAKE, None, None) };
    }
}

pub(crate) type WindowConstructor<'a, WindowData> = dyn FnMut(api::Window<()>) -> WindowData + 'a;

pub struct CreateStruct<'a, WindowData> {
    pub wndproc_state: *const (),
    pub constructor: *const RefCell<WindowConstructor<'a, WindowData>>,
    /// Place to stash any errors that may occur during window creation.
    pub error: Result<(), api::WindowError>,
    pub title: Option<Cow<'static, str>>,
    pub min_size: WindowSize,
    pub max_size: WindowSize,
    pub is_visible: bool,
    pub is_resizable: bool,
}

pub(crate) struct WindowState {
    pub title: Cow<'static, str>,
    pub size: WindowSize,
    pub min_size: WindowSize,
    pub max_size: WindowSize,
    pub position: WindowPoint,
    pub dpi: u16,
    pub has_focus: bool,
    pub is_visible: bool,
    pub has_pointer: bool,
    pub is_resizable: bool,
    pub is_resizing: bool,
    /// Keep this per-window, not per-event-loop because a different window
    /// might get a resize event while this one is still resizing. If that
    /// happens, we don't want the other window to get resize begin/end events.
    pub in_drag_resize: bool,
    pub paint_reason: Option<PaintReason>,
}

pub(crate) struct HandlerContext<'a, WindowData, H: EventHandler<WindowData>> {
    pub hwnd: &'a Cell<HWND>,
    pub data: &'a RefCell<MaybeUninit<WindowData>>,
    pub state: &'a RefCell<MaybeUninit<WindowState>>,
    pub event_handler: &'a RefCell<H>,
    pub event_loop: api::ActiveEventLoop<WindowData>,
}

impl<'a, WindowData, H: EventHandler<WindowData>> HandlerContext<'a, WindowData, H> {
    pub fn init(&mut self, hwnd: HWND, create_struct: &mut CreateStruct<WindowData>) {
        self.hwnd.set(hwnd);

        unsafe { SetLastError(WIN32_ERROR(0)) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.wndproc_state as _) };
        unsafe { GetLastError() }.expect("SetWindowLongPtrW(GWLP_USERDATA) failed.");

        self.state.borrow_mut().write({
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            assert!(dpi > 0, "GetDpiForWindow failed.");

            let mut default_rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut default_rect) }.expect("GetClientRect failed.");

            WindowState {
                title: unsafe { create_struct.title.take().unwrap_unchecked() },
                size: WindowSize {
                    width: (default_rect.right - default_rect.left) as _,
                    height: (default_rect.bottom - default_rect.top) as _,
                },
                max_size: create_struct.max_size,
                min_size: create_struct.min_size,
                position: WindowPoint {
                    x: default_rect.left,
                    y: default_rect.top,
                },
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

            unsafe { (*create_struct.constructor).borrow_mut()(window) }
        });
    }

    pub fn wake(&mut self) {
        self.event(|handler, event_loop, window| handler.wake_requested(event_loop, window));
    }

    pub fn close(&mut self) {
        self.event(|handler, event_loop, window| handler.close_requested(event_loop, window));
    }

    pub fn destroy_defer(&mut self) {
        unsafe { DestroyWindow(self.hwnd.get()) }.unwrap();
    }

    pub fn destroy(&mut self) {
        unsafe { SetWindowLongPtrW(self.hwnd.get(), GWLP_USERDATA, 0) };

        self.hwnd.set(HWND::default());

        // SAFETY: Clearing the hwnd marks the data as uninitialized. It will
        // not be read until it is reinitialized for a new window.
        let window_data = unsafe { self.data.borrow_mut().assume_init_read() };

        self.event_handler
            .borrow_mut()
            .destroyed(&self.event_loop, window_data);
    }

    pub fn show_defer(&mut self, show: SHOW_WINDOW_CMD) {
        unsafe { ShowWindow(self.hwnd.get(), show) };
    }

    pub fn show(&mut self, is_visible: bool) {
        self.with_state(|window| window.is_visible = is_visible);

        if is_visible {
            self.event(|handler, event_loop, window| handler.shown(event_loop, window));
        } else {
            self.event(|handler, event_loop, window| handler.hidden(event_loop, window));
        }
    }

    pub fn enter_size_move(&mut self) {
        self.with_state(|window| window.is_resizing = true);
    }

    pub fn exit_size_move(&mut self) {
        let resize_ended: bool = self.with_state(|window| {
            window.in_drag_resize = false;
            std::mem::take(&mut window.is_resizing)
        });

        if resize_ended {
            self.event(|handler, event_loop, window| handler.drag_resize_ended(event_loop, window));
        }
    }

    pub fn dpi_changed(&mut self, dpi: u16, rect: &RECT) {
        let size = WindowSize {
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        };

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

        let scale = DpiScale {
            factor: dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32,
        };

        self.event(|handler, event_loop, window| {
            handler.dpi_changed(event_loop, window, scale, size)
        });
    }

    pub fn get_min_max_info(&mut self, mmi: &mut MINMAXINFO) {
        let (min, max, dpi) =
            self.with_state(|window| (window.min_size, window.max_size, window.dpi as u32));

        let os_min_x = unsafe { GetSystemMetricsForDpi(SM_CXMINTRACK, dpi) };
        let os_min_y = unsafe { GetSystemMetricsForDpi(SM_CYMINTRACK, dpi) };
        let os_max_x = unsafe { GetSystemMetricsForDpi(SM_CXMAXTRACK, dpi) };
        let os_max_y = unsafe { GetSystemMetricsForDpi(SM_CYMAXTRACK, dpi) };

        mmi.ptMinTrackSize.x = min.width.max(os_min_x);
        mmi.ptMinTrackSize.y = min.height.max(os_min_y);
        mmi.ptMaxTrackSize.x = max.width.min(os_max_x);
        mmi.ptMaxTrackSize.y = max.height.min(os_max_y);
    }

    pub fn pos_changed(&mut self, pos: &WINDOWPOS) {
        let (x, y, width, height) = (pos.x, pos.y, pos.cx, pos.cy);

        let (resized, moved) = self.with_state(|window| {
            (
                {
                    let resized = width != window.size.width || height != window.size.height;
                    let is_start = resized && window.in_drag_resize;

                    window.size = WindowSize { width, height };
                    window.is_resizing = is_start;
                    window.paint_reason = resized
                        .then_some(PaintReason::Commanded) // override if resized
                        .or(window.paint_reason); // else keep the current reason

                    resized.then_some((is_start, window.size))
                },
                {
                    let moved = x != window.position.x || y != window.position.y;

                    window.position = WindowPoint { x, y }; // doesn't matter if it's the same

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

    pub fn defer_paint(&mut self) {
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

    pub fn paint(&mut self) {
        let mut ps = PAINTSTRUCT::default();

        let is_invalid = unsafe { BeginPaint(self.hwnd.get(), &mut ps) }.is_invalid();
        assert!(!is_invalid, "BeginPaint failed.");

        unsafe { EndPaint(self.hwnd.get(), &ps) };

        let reason = self
            .with_state(|window| window.paint_reason.take())
            .unwrap_or(PaintReason::Commanded); // assume no reason means it's from the OS

        self.event(|handler, event_loop, window| handler.needs_repaint(event_loop, window, reason));
    }

    pub fn mouse_move(&mut self, position: WindowPoint) {
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

    pub fn mouse_leave(&mut self) {
        self.with_state(|window| window.has_pointer = false);
        self.event(|handler, event_loop, window| handler.pointer_left(event_loop, window));
    }

    pub fn wm_mouse_wheel(&mut self, axis: ScrollAxis, delta: f32, mods: ModifierKeys) {
        self.event(|handler, event_loop, window| {
            handler.mouse_scrolled(event_loop, window, delta, axis, mods)
        });
    }

    pub fn mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        position: WindowPoint,
        mods: ModifierKeys,
    ) {
        self.event(|handler, event_loop, window| {
            handler.mouse_button(event_loop, window, button, state, position, mods)
        });
    }

    #[inline]
    pub fn with_state<T>(&mut self, f: impl FnOnce(&mut WindowState) -> T) -> T {
        assert_ne!(self.hwnd.get(), HWND::default(), "Window not initialized.");

        let mut state = self.state.borrow_mut();
        let state = unsafe { state.assume_init_mut() };
        f(state)
    }

    #[inline]
    pub fn event(
        &mut self,
        f: impl FnOnce(&mut H, &api::ActiveEventLoop<WindowData>, api::Window<WindowData>),
    ) {
        assert_ne!(self.hwnd.get(), HWND::default(), "Window not initialized.");

        let (state, mut data) = (self.state.borrow(), self.data.borrow_mut());
        let mut handler = self.event_handler.borrow_mut();
        let window = api::Window {
            window: Window {
                hwnd: self.hwnd.get(),
                state: unsafe { state.assume_init_ref() },
                data: unsafe { data.assume_init_mut() },
                _phantom: PhantomData,
            },
        };

        f(&mut *handler, &self.event_loop, window);
    }
}

pub struct Window<'a, Data> {
    hwnd: HWND,
    state: &'a WindowState,
    data: &'a mut Data,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, Data> Window<'a, Data> {
    pub fn waker(&self) -> api::WindowWaker {
        api::WindowWaker {
            waker: WindowWaker { target: self.hwnd },
        }
    }

    pub fn destroy(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DEFER_DESTROY, None, None) }.unwrap();
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn data(&self) -> &Data {
        self.data
    }

    pub fn data_mut(&mut self) -> &mut Data {
        self.data
    }

    pub fn map<Data2>(
        self,
        f: impl FnOnce(&'a mut Data) -> &'a mut Data2,
    ) -> api::Window<'a, Data2> {
        api::Window {
            window: Window {
                hwnd: self.hwnd,
                state: self.state,
                data: f(self.data),
                _phantom: PhantomData,
            },
        }
    }

    pub fn title(&self) -> &str {
        &self.state.title
    }

    #[allow(unused_variables)]
    pub fn set_title(&mut self, title: &str) {
        todo!()
    }

    pub fn size(&self) -> WindowSize {
        self.state.size
    }

    #[allow(unused_variables)]
    pub fn set_size(&mut self, size: WindowSize) {
        todo!()
    }

    pub fn min_size(&self) -> WindowSize {
        self.state.min_size
    }

    pub fn set_min_size(&mut self, min_size: WindowSize) {
        todo!()
    }

    pub fn max_size(&self) -> WindowSize {
        self.state.max_size
    }

    pub fn set_max_size(&mut self, max_size: WindowSize) {
        todo!()
    }

    pub fn position(&self) -> WindowPoint {
        self.state.position
    }

    #[allow(unused_variables)]
    pub fn set_position(&mut self, position: WindowPoint) {
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
        let factor = self.state.dpi as f32 / USER_DEFAULT_SCREEN_DPI as f32;
        DpiScale { factor }
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

impl<'a, Meta, User> Window<'a, (Meta, User)> {
    pub fn split(self) -> (&'a mut Meta, api::Window<'a, User>) {
        let (meta, user) = self.data;
        (
            meta,
            api::Window {
                window: Window {
                    hwnd: self.hwnd,
                    state: self.state,
                    data: user,
                    _phantom: PhantomData,
                },
            },
        )
    }
}

pub(crate) fn register_wndclass(
    wndproc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> windows::core::Result<PCWSTR> {
    let atom = unsafe {
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
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
        unsafe { GetLastError() }?;
    }

    Ok(PCWSTR(atom as usize as *const _))
}

/// Add a message to the queue to show or hide a window. Necessary because
/// `ShowWindow` is synchronous, which can call the wndproc re-entrantly.
pub fn post_defer_show(hwnd: HWND, show: SHOW_WINDOW_CMD) {
    unsafe { PostMessageW(hwnd, UM_DEFER_SHOW, None, LPARAM(show.0 as _)) }.unwrap();
}

/// Extract the `SHOW_WINDOW_CMD` from the `LPARAM` of a `UM_DEFER_SHOW`
/// message.
pub fn from_defer_show(lparam: LPARAM) -> SHOW_WINDOW_CMD {
    SHOW_WINDOW_CMD(lparam.0 as i32)
}
