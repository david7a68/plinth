use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::addr_of,
};

use arrayvec::ArrayVec;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::HBRUSH,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::WM_MOUSELEAVE,
            HiDpi::{
                SetProcessDpiAwareness, SetProcessDpiAwarenessContext,
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, PROCESS_PER_MONITOR_DPI_AWARE,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
                LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassExW, TranslateMessage,
                CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HICON,
                IDC_ARROW, MSG, PM_NOREMOVE, SW_NORMAL, WM_CLOSE, WM_CREATE, WM_DESTROY,
                WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_LBUTTONDOWN,
                WM_MBUTTONDBLCLK, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT,
                WM_SHOWWINDOW, WM_TIMER, WM_WINDOWPOSCHANGED, WNDCLASSEXW,
                WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

const WND_CLASS_NAME: PCWSTR = w!("plinth_wc");

use crate::{
    geometry::Extent,
    limits::{MAX_WINDOWS, MAX_WINDOW_TITLE_LENGTH},
};

use super::{
    api, input,
    window::{
        from_defer_show, post_defer_show, CreateStruct, HandlerContext, WindowError, WindowState,
        UM_DEFER_DESTROY, UM_DEFER_PAINT, UM_DEFER_SHOW, UM_WAKE,
    },
};

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
        attributes: api::WindowAttributes,
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
        let max_size = attributes.max_size.unwrap_or(Extent::MAX);

        let (width, height) = attributes.size.map_or((CW_USEDEFAULT, CW_USEDEFAULT), |s| {
            (i32::from(s.width), i32::from(s.height))
        });

        let (x, y) = attributes
            .position
            .map_or((CW_USEDEFAULT, CW_USEDEFAULT), |p| {
                (i32::from(p.x), i32::from(p.y))
            });

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
                Some(addr_of!(create_struct).cast()),
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
    #[allow(clippy::unnecessary_wraps)] // for consistency with other platforms
    pub fn new() -> Result<Self, EventLoopError> {
        Ok(Self {})
    }

    #[allow(clippy::unused_self)]
    pub fn run<WindowData, H: api::EventHandler<WindowData>>(
        &mut self,
        event_handler: H,
    ) -> Result<(), api::EventLoopError> {
        Self::configure_env()?;

        let wndclass = register_wndclass(unsafe_wndproc::<WindowData, H>)
            .map_err(EventLoopError::RegisterClassFailed)?;

        let wndproc_state = WndProcState::<WindowData, H> {
            wndclass,
            event_handler: RefCell::new(event_handler),

            in_size_move: Cell::new(false),

            hwnds: [(); MAX_WINDOWS].map(|()| Cell::new(HWND::default())),
            window_data: [(); MAX_WINDOWS].map(|()| RefCell::new(MaybeUninit::uninit())),
            window_states: [(); MAX_WINDOWS].map(|()| RefCell::new(MaybeUninit::uninit())),
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

struct WndProcState<WindowData, H: api::EventHandler<WindowData>> {
    wndclass: PCWSTR,
    event_handler: RefCell<H>,

    in_size_move: Cell<bool>,

    hwnds: [Cell<HWND>; MAX_WINDOWS],
    window_data: [RefCell<MaybeUninit<WindowData>>; MAX_WINDOWS],
    window_states: [RefCell<MaybeUninit<WindowState>>; MAX_WINDOWS],
}

impl<WindowData, H: api::EventHandler<WindowData>> WndProcState<WindowData, H> {
    fn as_active_event_loop(&self) -> api::ActiveEventLoop<WindowData> {
        api::ActiveEventLoop {
            event_loop: ActiveEventLoop {
                wndclass: self.wndclass,
                opaque_state: (self as *const WndProcState<_, _>).cast(),
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

unsafe extern "system" fn unsafe_wndproc<WindowData, H: api::EventHandler<WindowData>>(
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
            let cs: &CREATESTRUCTW = cast_lparam_as_ref(lparam);
            let cs = &*(cs.lpCreateParams as *const RefCell<CreateStruct<WindowData>>);
            cs.borrow_mut()
        };

        let state = &*cs.wndproc_state.cast::<WndProcState<WindowData, H>>();

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

            unsafe { &*state.cast::<WndProcState<WindowData, H>>() }
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
                    count += u32::from(h.get() != HWND::default()); // branchless count
                }

                if count == 0 {
                    unsafe { PostQuitMessage(0) };
                }
            }
            UM_DEFER_SHOW => context.show_defer(from_defer_show(lparam)),
            WM_SHOWWINDOW => context.show(wparam.0 != 0),
            WM_ENTERSIZEMOVE => {
                assert!(
                    !state.in_size_move.get(),
                    "WM_ENTERSIZEMOVE while already in WM_ENTERSIZEMOVE"
                );
                state.in_size_move.set(true);
                context.modal_loop_enter();
            }
            WM_EXITSIZEMOVE => {
                assert!(
                    state.in_size_move.get(),
                    "WM_EXITSIZEMOVE without WM_ENTERSIZEMOVE"
                );
                state.in_size_move.set(false);
                context.modal_loop_leave();
            }
            WM_DPICHANGED => {
                let dpi = u16::try_from(wparam.0).expect("WM_DPICHANGED exceeded u16::MAX");
                context.dpi_changed(dpi, cast_lparam_as_ref(lparam));
            }
            WM_GETMINMAXINFO => context.get_min_max_info(cast_lparam_as_mut(lparam)),
            WM_WINDOWPOSCHANGED => context.pos_changed(cast_lparam_as_ref(lparam)),
            UM_DEFER_PAINT => context.paint_defer(),
            WM_PAINT => context.paint(),
            WM_MOUSEMOVE => context.mouse_move(input::mouse_coords(lparam)),
            WM_MOUSELEAVE => context.mouse_leave(),
            msg @ (WM_MOUSEWHEEL | WM_MOUSEHWHEEL) => {
                let axis = input::wheel_axis(msg).unwrap();

                #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
                let delta = f32::from((wparam.0 >> 16) as i16) / 120.0;

                let mods = input::mouse_modifiers(wparam);
                context.mouse_wheel(axis, delta, mods);
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

fn cast_lparam_as_ref<T>(lparam: LPARAM) -> &'static T {
    unsafe { &*(lparam.0 as *const T) }
}

fn cast_lparam_as_mut<T>(lparam: LPARAM) -> &'static mut T {
    unsafe { &mut *(lparam.0 as *mut T) }
}

pub(crate) fn register_wndclass(
    wndproc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> windows::core::Result<PCWSTR> {
    let atom = unsafe {
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: u32::try_from(std::mem::size_of::<WNDCLASSEXW>()).unwrap(),
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
