use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::MaybeUninit,
};

use limits::WindowTitle;
use windows::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, LPARAM, RECT, WIN32_ERROR},
    Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT},
    UI::{
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
        WindowsAndMessaging::{
            DestroyWindow, GetClientRect, PostMessageW, SetWindowLongPtrW, SetWindowPos,
            ShowWindow, GWLP_USERDATA, MINMAXINFO, SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD,
            SM_CXMAXTRACK, SM_CXMINTRACK, SM_CYMAXTRACK, SM_CYMINTRACK, SW_HIDE, SW_NORMAL,
            USER_DEFAULT_SCREEN_DPI, WINDOWPOS, WM_APP,
        },
    },
};

use crate::{
    core::limit::Limit,
    limits,
    system::{
        event_loop::{Event, Handler, WindowEvent},
        input::{ButtonState, ModifierKeys, MouseButton, ScrollAxis},
        window::{PaintReason, RefreshRateRequest},
        DpiScale, WindowExtent, WindowPoint,
    },
    time::FramesPerSecond,
};

use super::api;

pub(crate) const UM_WAKE: u32 = WM_APP;
pub(crate) const UM_DEFER_DESTROY: u32 = WM_APP + 1;
pub(crate) const UM_DEFER_SHOW: u32 = WM_APP + 2;
/// Message used to request a repaint. This is used instead of directly calling
/// `InvalidateRect` so as to consolidate repaint logic to the event loop. This
/// is safe to do since the event loop will not generate `WM_PAINT` events until
/// the message queue is empty.
///
/// This is slightly less efficient since we need to round-trip into the message
/// queue, but the simplicity was deemed worth it.
pub(crate) const UM_DEFER_PAINT: u32 = WM_APP + 3;

#[allow(clippy::cast_possible_truncation)]
const DEFAULT_DPI: u16 = USER_DEFAULT_SCREEN_DPI as u16;

#[derive(Clone, Debug, thiserror::Error)]
pub enum WindowError {
    #[error("Window creation failed: {0:?}")]
    CreateFailed(windows::core::Error),
}

pub struct Waker {
    target: HWND,
}

impl Waker {
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
    pub min_size: WindowExtent,
    pub max_size: WindowExtent,
    pub is_visible: bool,
    pub is_resizable: bool,
}

bitflags::bitflags! {
    pub(crate) struct WindowFlags: u8 {
        const IS_VISIBLE = 0b0000_0001;
        const IS_RESIZABLE = 0b0000_0010;
        const HAS_FOCUS = 0b0000_0100;
        const HAS_POINTER = 0b0000_1000;
        const IS_RESIZING = 0b0001_0000;
        /// Keep this per-window, not per-event-loop because a different window
        /// might get a resize event while this one is still resizing. If that
        /// happens, we don't want the other window to get resize begin/end events.
        const IN_DRAG_RESIZE = 0b0010_0000;
    }
}

pub(crate) struct WindowState {
    pub title: Cow<'static, str>,
    pub size: WindowExtent,
    pub min_size: WindowExtent,
    pub max_size: WindowExtent,
    pub position: WindowPoint,
    pub dpi: u16,
    pub flags: WindowFlags,
    pub paint_reason: Option<PaintReason>,
}

pub(crate) struct HandlerContext<'a, WindowData, H: Handler<WindowData>> {
    pub hwnd: &'a Cell<HWND>,
    pub data: &'a RefCell<MaybeUninit<WindowData>>,
    pub state: &'a RefCell<MaybeUninit<WindowState>>,
    pub event_handler: &'a RefCell<H>,
    pub event_loop: api::ActiveEventLoop<WindowData>,
}

impl<'a, WindowData, H: Handler<WindowData>> HandlerContext<'a, WindowData, H> {
    pub fn init(&mut self, hwnd: HWND, create_struct: &mut CreateStruct<WindowData>) {
        WindowTitle::new(create_struct.title.as_ref().unwrap()).clamp();

        create_struct.min_size.limit_assert();
        create_struct.max_size.limit_assert();

        self.hwnd.set(hwnd);

        unsafe { SetLastError(WIN32_ERROR(0)) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.wndproc_state as _) };
        unsafe { GetLastError() }
            .ok()
            .expect("SetWindowLongPtrW(GWLP_USERDATA) failed.");

        self.state.borrow_mut().write({
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            assert!(dpi > 0, "GetDpiForWindow failed.");

            let (size, position) = {
                let mut default_rect = RECT::default();
                unsafe { GetClientRect(hwnd, &mut default_rect) }.expect("GetClientRect failed.");
                unpack_rect(&default_rect)
            };

            let mut flags = WindowFlags::empty();

            if create_struct.is_visible {
                flags |= WindowFlags::IS_VISIBLE;
            }

            if create_struct.is_resizable {
                flags |= WindowFlags::IS_RESIZABLE;
            }

            WindowState {
                title: unsafe { create_struct.title.take().unwrap_unchecked() },
                size,
                max_size: create_struct.max_size,
                min_size: create_struct.min_size,
                position,
                dpi: u16::try_from(dpi).unwrap(),
                flags,
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
        self.event(WindowEvent::Wake);
    }

    pub fn close(&mut self) {
        self.event(WindowEvent::CloseRequest);
    }

    pub fn destroy_defer(&mut self) {
        unsafe { DestroyWindow(self.hwnd.get()) }.unwrap();
    }

    pub fn destroy(&mut self) {
        self.event(WindowEvent::Destroy);
        unsafe { SetWindowLongPtrW(self.hwnd.get(), GWLP_USERDATA, 0) };
        self.hwnd.set(HWND::default());
    }

    pub fn show_defer(&mut self, show: SHOW_WINDOW_CMD) {
        let _ = unsafe { ShowWindow(self.hwnd.get(), show) };
    }

    pub fn show(&mut self, is_visible: bool) {
        self.with_state(|window| window.flags.set(WindowFlags::IS_VISIBLE, is_visible));

        if is_visible {
            self.event(WindowEvent::Shown);
        } else {
            self.event(WindowEvent::Hidden);
        }
    }

    pub fn modal_loop_enter(&mut self) {
        // This may be called more than once without first receiving a matching
        // WM_EXITSIZEMOVE under certain conditions, so this operation must be
        // idempotent. Not sure _why_ it's called more than once.
        //
        // -dz (2024-03-03)
        self.with_state(|window| window.flags.set(WindowFlags::IN_DRAG_RESIZE, true));
    }

    pub fn modal_loop_leave(&mut self) {
        let resize_ended: bool = self.with_state(|window| {
            window.flags.remove(WindowFlags::IN_DRAG_RESIZE);

            let ended = window.flags.contains(WindowFlags::IS_RESIZING);
            window.flags.set(WindowFlags::IS_RESIZING, false);
            ended
        });

        if resize_ended {
            self.event(WindowEvent::DragResize(false));
        }
    }

    pub fn dpi_changed(&mut self, dpi: u16, rect: &RECT) {
        let (size, _) = unpack_rect(rect);

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

        let scale = DpiScale::new(f32::from(dpi) / f32::from(DEFAULT_DPI));

        self.event(WindowEvent::DpiChange(scale, size));
    }

    pub fn get_min_max_info(&mut self, mmi: &mut MINMAXINFO) {
        let (min, max, dpi) =
            self.with_state(|window| (window.min_size, window.max_size, u32::from(window.dpi)));

        debug_assert!(min.width <= max.width && min.height <= max.height);
        debug_assert!(dpi >= DEFAULT_DPI as u32);

        let min_x = (min.width as i32).max(unsafe { GetSystemMetricsForDpi(SM_CXMINTRACK, dpi) });
        let min_y = (min.height as i32).max(unsafe { GetSystemMetricsForDpi(SM_CYMINTRACK, dpi) });
        let max_x = (max.width as i32).min(unsafe { GetSystemMetricsForDpi(SM_CXMAXTRACK, dpi) });
        let max_y = (max.height as i32).min(unsafe { GetSystemMetricsForDpi(SM_CYMAXTRACK, dpi) });

        debug_assert!(min_x <= max_x && min_y <= max_y);

        mmi.ptMinTrackSize.x = min_x;
        mmi.ptMinTrackSize.y = min_y;
        mmi.ptMaxTrackSize.x = max_x;
        mmi.ptMaxTrackSize.y = max_y;
    }

    pub fn pos_changed(&mut self, pos: &WINDOWPOS) {
        let x = i16::try_from(pos.x).unwrap();
        let y = i16::try_from(pos.y).unwrap();
        let width = i16::try_from(pos.cx).unwrap();
        let height = i16::try_from(pos.cy).unwrap();

        let (resized, moved) = self.with_state(|window| {
            let resized = {
                let resized = width != window.size.width || height != window.size.height;
                let is_start = resized && window.flags.contains(WindowFlags::IN_DRAG_RESIZE);

                window.size = WindowExtent::new(width, height);
                window.flags.set(WindowFlags::IS_RESIZING, true);
                window.paint_reason = resized
                    .then_some(PaintReason::Commanded) // override if resized
                    .or(window.paint_reason); // else keep the current reason

                resized.then_some((is_start, window.size))
            };

            let moved = {
                let moved = x != window.position.x || y != window.position.y;
                window.position = WindowPoint { x, y };
                moved.then_some(window.position)
            };

            (resized, moved)
        });

        if let Some((start, size)) = resized {
            if start {
                self.event(WindowEvent::DragResize(true));
            }

            self.event(WindowEvent::Resize(size));
        }

        if let Some(pos) = moved {
            self.event(WindowEvent::Move(pos));
        }
    }

    pub fn paint_defer(&mut self) {
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

        let _ = unsafe { EndPaint(self.hwnd.get(), &ps) };

        let reason = self
            .with_state(|window| window.paint_reason.take())
            .unwrap_or(PaintReason::Commanded); // assume no reason means it's from the OS

        self.event(WindowEvent::Repaint(reason));
    }

    pub fn mouse_move(&mut self, position: WindowPoint) {
        let entered = self.with_state(|window| {
            let has_pointer = window.flags.contains(WindowFlags::HAS_POINTER);
            window.flags.set(WindowFlags::HAS_POINTER, true);
            !has_pointer
        });

        if entered {
            self.event(WindowEvent::PointerEntered(position));

            unsafe {
                TrackMouseEvent(&mut TRACKMOUSEEVENT {
                    cbSize: u32::try_from(std::mem::size_of::<TRACKMOUSEEVENT>()).unwrap(),
                    dwFlags: TME_LEAVE,
                    hwndTrack: self.hwnd.get(),
                    dwHoverTime: 0,
                })
            }
            .unwrap();
        }

        self.event(WindowEvent::PointerMoved(position));
    }

    pub fn mouse_leave(&mut self) {
        self.with_state(|window| window.flags.set(WindowFlags::HAS_POINTER, false));
        self.event(WindowEvent::PointerLeft);
    }

    pub fn mouse_wheel(&mut self, axis: ScrollAxis, delta: f32, mods: ModifierKeys) {
        self.event(WindowEvent::MouseScrolled(delta, axis, mods));
    }

    pub fn mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        position: WindowPoint,
        mods: ModifierKeys,
    ) {
        self.event(WindowEvent::MouseButton(button, state, position, mods));
    }

    #[inline]
    pub fn with_state<T>(&mut self, f: impl FnOnce(&mut WindowState) -> T) -> T {
        assert_ne!(self.hwnd.get(), HWND::default(), "Window not initialized.");

        let mut state = self.state.borrow_mut();
        let state = unsafe { state.assume_init_mut() };
        f(state)
    }

    pub fn event(&mut self, event: WindowEvent) {
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

        (*handler).handle(&self.event_loop, Event::Window(window, event));
    }
}

pub struct Window<'a, Data> {
    hwnd: HWND,
    state: &'a WindowState,
    data: &'a mut Data,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, Data> Window<'a, Data> {
    pub fn waker(&self) -> api::Waker {
        api::Waker {
            waker: Waker { target: self.hwnd },
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

    pub fn title(&self) -> &str {
        &self.state.title
    }

    #[allow(unused_variables)]
    pub fn set_title(&mut self, title: &str) {
        todo!()
    }

    pub fn size(&self) -> WindowExtent {
        self.state.size
    }

    #[allow(unused_variables)]
    pub fn set_size(&mut self, size: WindowExtent) {
        todo!()
    }

    pub fn min_size(&self) -> WindowExtent {
        self.state.min_size
    }

    pub fn set_min_size(&mut self, min_size: WindowExtent) {
        let _ = min_size;
        todo!()
    }

    pub fn max_size(&self) -> WindowExtent {
        self.state.max_size
    }

    pub fn set_max_size(&mut self, max_size: WindowExtent) {
        let _ = max_size;
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
        self.state.flags.contains(WindowFlags::IS_VISIBLE)
    }

    pub fn show(&mut self) {
        post_defer_show(self.hwnd, SW_NORMAL);
    }

    pub fn hide(&mut self) {
        post_defer_show(self.hwnd, SW_HIDE);
    }

    pub fn is_resizable(&self) -> bool {
        self.state.flags.contains(WindowFlags::IS_RESIZABLE)
    }

    pub fn dpi_scale(&self) -> DpiScale {
        let factor = f32::from(self.state.dpi) / f32::from(DEFAULT_DPI);
        DpiScale::new(factor)
    }

    pub fn has_focus(&self) -> bool {
        self.state.flags.contains(WindowFlags::HAS_FOCUS)
    }

    pub fn has_pointer(&self) -> bool {
        self.state.flags.contains(WindowFlags::HAS_POINTER)
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

impl<'a, Data> Window<'a, Option<Data>> {
    pub fn extract_option(self) -> Option<Window<'a, Data>> {
        match self.data {
            Some(data) => Some(Window {
                hwnd: self.hwnd,
                state: self.state,
                data,
                _phantom: PhantomData,
            }),
            None => None,
        }
    }
}

fn unpack_rect(rect: &RECT) -> (WindowExtent, WindowPoint) {
    let point = WindowPoint {
        x: i16::try_from(rect.left).unwrap(),
        y: i16::try_from(rect.top).unwrap(),
    };

    let extent = WindowExtent {
        width: i16::try_from(rect.right - rect.left).unwrap(),
        height: i16::try_from(rect.bottom - rect.top).unwrap(),
    };

    (extent, point)
}

/// Add a message to the queue to show or hide a window. Necessary because
/// `ShowWindow` is synchronous, which can call the wndproc re-entrantly.
pub fn post_defer_show(hwnd: HWND, show: SHOW_WINDOW_CMD) {
    unsafe { PostMessageW(hwnd, UM_DEFER_SHOW, None, LPARAM(show.0 as _)) }.unwrap();
}

/// Extract the `SHOW_WINDOW_CMD` from the `LPARAM` of a `UM_DEFER_SHOW`
/// message.
pub fn from_defer_show(lparam: LPARAM) -> SHOW_WINDOW_CMD {
    #[allow(clippy::cast_possible_truncation)]
    SHOW_WINDOW_CMD(lparam.0 as _)
}
