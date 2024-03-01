mod input;
pub mod time;
mod window;

pub use window::*;

use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::MaybeUninit,
};

use arrayvec::ArrayVec;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Controls::WM_MOUSELEAVE,
            HiDpi::{
                SetProcessDpiAwareness, SetProcessDpiAwarenessContext,
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, PROCESS_PER_MONITOR_DPI_AWARE,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
                PeekMessageW, PostQuitMessage, TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT,
                GWLP_USERDATA, MSG, PM_NOREMOVE, SW_NORMAL, WM_CLOSE, WM_CREATE, WM_DESTROY,
                WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_LBUTTONDOWN,
                WM_MBUTTONDBLCLK, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
                WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED, WS_EX_NOREDIRECTIONBITMAP,
                WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use crate::{
    geometry::window::WindowSize,
    limits::{MAX_WINDOWS, MAX_WINDOW_TITLE_LENGTH},
};

use super::{event_loop::EventHandler, window::WindowAttributes};

mod api {
    pub use crate::system::event_loop::{ActiveEventLoop, EventLoopError};
    pub use crate::system::window::{Window, WindowError, WindowWaker};
}

#[derive(Debug, thiserror::Error)]
pub enum EventLoopError {
    #[error("[likely bug] An error occurred while registering the window class: {0}")]
    RegisterClassFailed(windows::core::Error),

    #[error("An OS error has occurred. This is likely a bug. {0}")]
    Internal(windows::core::Error),
}

pub struct ActiveEventLoop<WindowData> {
    wndclass: PCWSTR,
    opaque_state: *const (),
    _phantom: PhantomData<*const WindowData>,
}

impl<WindowData> ActiveEventLoop<WindowData> {
    pub fn create_window<F: FnOnce(api::Window<()>) -> WindowData>(
        &self,
        attributes: WindowAttributes,
        constructor: F,
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
        let max_size = attributes.max_size.unwrap_or(WindowSize::MAX);

        let (width, height) = attributes
            .size
            .map(|s| (s.width, s.height))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let (x, y) = attributes
            .position
            .map(|p| (p.x, p.y))
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let mut opt = Some(constructor);
        let wrap_ctor = RefCell::new(|window: api::Window<()>| opt.take().unwrap()(window));

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
        Self::configure_env()?;

        let wndclass = register_wndclass(unsafe_wndproc::<WindowData, H>)
            .map_err(EventLoopError::RegisterClassFailed)?;

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

    fn configure_env() -> Result<(), api::EventLoopError> {
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

        Ok(())
    }
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

        let mut cs = {
            // let cs = &*(lparam.0 as *const CREATESTRUCTW);
            let cs: &CREATESTRUCTW = lparam_as_ref(lparam);
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
                    count += (h.get() != HWND::default()) as u32; // branchless count
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
                context.dpi_changed(u16::try_from(wparam.0).unwrap(), lparam_as_ref(lparam));
            }
            WM_GETMINMAXINFO => context.get_min_max_info(lparam_as_mut(lparam)),
            WM_WINDOWPOSCHANGED => context.pos_changed(lparam_as_ref(lparam)),
            UM_DEFER_PAINT => context.defer_paint(),
            WM_PAINT => context.paint(),
            WM_MOUSEMOVE => context.mouse_move(input::mouse_coords(lparam)),
            WM_MOUSELEAVE => context.mouse_leave(),
            msg @ (WM_MOUSEWHEEL | WM_MOUSEHWHEEL) => {
                let axis = input::wheel_axis(msg).unwrap();

                #[allow(clippy::cast_possible_wrap)]
                let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;

                let mods = input::mouse_modifiers(wparam);
                context.wm_mouse_wheel(axis, delta, mods);
            }
            msg @ WM_LBUTTONDOWN..=WM_MBUTTONDBLCLK => {
                let (button, state) = input::mouse_button(msg).unwrap();
                let point = input::mouse_coords(lparam);
                let mods = input::mouse_modifiers(wparam);

                context.mouse_button(button, state, point, mods);
            }
            // TODO: keyboard input
            _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        };

        LRESULT(0)
    }
}

fn lparam_as_ref<T>(lparam: LPARAM) -> &'static T {
    unsafe { &*(lparam.0 as *const T) }
}

fn lparam_as_mut<T>(lparam: LPARAM) -> &'static mut T {
    unsafe { &mut *(lparam.0 as *mut T) }
}
