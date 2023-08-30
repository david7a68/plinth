use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::OnceLock,
};

use arrayvec::ArrayVec;
use euclid::Size2D;

use windows::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW, PostMessageW,
            PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow, ShowWindowAsync,
            TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA,
            HICON, HMENU, IDC_ARROW, MSG, PM_NOREMOVE, SW_SHOW, WM_CLOSE, WM_CREATE, WM_DESTROY,
            WM_ENTERSIZEMOVE, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_PAINT, WM_TIMER,
            WM_USER, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

pub const MAX_TITLE_LENGTH: usize = 256;

/// Represents measurement units in pixels before any DPI scaling is applied.
pub struct ScreenSpace;

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

const UM_DESTROY_WINDOW: u32 = WM_USER;

// Global state
static WND_CLASS_ATOM: OnceLock<u16> = OnceLock::new(); // TODO: make thread local?

fn wnd_class_atom_as_pcwstr() -> PCWSTR {
    PCWSTR(WND_CLASS_ATOM.get().unwrap().clone() as usize as *const _)
}

thread_local! {
    static NUM_WINDOWS: Cell<usize> = const { Cell::new(0) };
    static EVENT_SINK: RefCell<Option<Box<dyn EventSink>>> = RefCell::default();
}

/// Registers the window class for the application.
///
/// This is a one-time operation. Multiple calls to this function will not
/// re-register the class.
fn register_window_class_once() {
    WND_CLASS_ATOM.get_or_init(|| {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc_trampoline),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: unsafe { GetModuleHandleW(None) }.unwrap(),
            hIcon: HICON::default(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: CLASS_NAME,
            hIconSm: HICON::default(),
        };

        let atom = unsafe { RegisterClassExW(&class) };

        if atom == 0 {
            panic!(
                "Failed to register window class, error code: {}",
                unsafe { GetLastError() }.0
            );
        } else {
            tracing::info!("Registered window class");
        }

        atom
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowId(pub u64);

pub trait EventSink {
    fn send(&mut self, window: WindowId, event: Event);
}

pub enum Event {
    Create(WindowHandle),
    Destroy,
    Close,
    ResizeBegin,
    ResizeEnd,
    Paint(Size2D<u16, ScreenSpace>),
}

/// Specifies the properties of a window.
pub struct WindowSpec<'a> {
    pub title: &'a str,
    /// The size of the window's content area (excluding borders and title bar).
    pub size: Size2D<u16, ScreenSpace>,
}

impl<'a> WindowSpec<'a> {
    /// Constructs a new window with the specified properties.
    ///
    /// The event handler determines how the window responds to OS events.
    pub fn build(&self, source_id: WindowId) -> WindowHandle {
        register_window_class_once();

        let title = translate_title(self.title);

        let handle = Rc::default();
        let handle_weak = Rc::downgrade(&handle);

        let inner_state = Box::new(WindowState {
            id: source_id,
            hwnd: handle,
        });

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: self.size.width.into(),
            bottom: self.size.height.into(),
        };

        unsafe { AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, false) };

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                // Use the atom for later comparison. This way we don't have to
                // compare c-style strings.
                wnd_class_atom_as_pcwstr(),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right,
                rect.bottom,
                HWND::default(),
                HMENU::default(),
                HMODULE::default(),
                Some(Box::into_raw(inner_state).cast()),
            )
        };

        if hwnd.0 == 0 {
            panic!(
                "Failed to create window, error code: {}",
                unsafe { GetLastError() }.0
            );
        } else {
            tracing::info!("Created window");
        }

        unsafe { ShowWindow(hwnd, SW_SHOW) };

        WindowHandle { inner: handle_weak }
    }
}

fn translate_title(title: &str) -> ArrayVec<u16, { MAX_TITLE_LENGTH + 1 }> {
    if title.len() > MAX_TITLE_LENGTH {
        tracing::warn!(
            "Window title is too long, truncating to {} characters",
            MAX_TITLE_LENGTH
        );
    }

    let mut title: ArrayVec<u16, { MAX_TITLE_LENGTH + 1 }> =
        title.encode_utf16().take(MAX_TITLE_LENGTH).collect();
    title.push(0);
    title
}

#[derive(Clone)]
pub struct WindowHandle {
    // This is a weak reference to the window state, which is owned by the OS.
    // Since the OS will never destroy the window without our input, this should
    // be safe. If it isn't, the program will panic.
    inner: Weak<Cell<HWND>>,
}

impl WindowHandle {
    pub fn hwnd(&self) -> HWND {
        if let Some(inner) = self.inner.upgrade() {
            inner.get()
        } else {
            panic!("Window was destroyed")
        }
    }

    pub fn content_size(&self) -> Size2D<u16, ScreenSpace> {
        if let Some(inner) = self.inner.upgrade() {
            let mut client_rect = RECT::default();
            unsafe {
                GetClientRect(inner.get(), &mut client_rect);
            }
            Size2D::new(client_rect.right, client_rect.bottom)
                .try_cast::<u16>()
                .expect("Window size is negative or larger than u16::MAX")
        } else {
            panic!("Window was destroyed")
        }
    }

    pub fn show(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let hwnd = inner.get();
            unsafe { ShowWindowAsync(hwnd, SW_SHOW) };
        }
    }

    pub fn destroy(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let hwnd = inner.get();
            unsafe { PostMessageW(hwnd, UM_DESTROY_WINDOW, None, None) };
        }
    }
}

struct WindowState {
    id: WindowId,
    hwnd: Rc<Cell<HWND>>,
}

pub struct EventLoop {}

impl EventLoop {
    /// Hijacks the current thread to run the event loop. The thread will
    /// terminate once all windows are destroyed.
    ///
    /// All windows must be created on the same thread that runs the event loop,
    /// or they will not receive events.
    pub fn run<S: EventSink + Sized + 'static>(
        event_sink: S,
        mut init_with_event_loop: impl FnMut(),
    ) {
        EVENT_SINK.with(|s| {
            s.borrow_mut().replace(Box::new(event_sink));
        });

        init_with_event_loop();

        loop {
            let mut msg = MSG::default();

            // Force any pending timer messages to be generated. This is in case
            // the message queue keeps getting higher priority messages faster
            // than it can process them.
            unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

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
                _ => unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }

        // Allow the event sink to be dropped.
        EVENT_SINK.with(|s| s.borrow_mut().take());
    }
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTW;

        if (*create_struct).lpszClass != wnd_class_atom_as_pcwstr() {
            // Compare against the class atom instead of trying to compare c strings.

            // This is not a window created by us. I have no idea how this could
            // happen, but just in case...

            tracing::warn!("Window created with unknown class name. Ignoring.");
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }

        let window_state = (*create_struct).lpCreateParams as *mut WindowState;

        (*window_state).hwnd.set(hwnd);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_state as _);

        NUM_WINDOWS.with(|n| n.set(n.get() + 1));

        tracing::debug!(
            "Window created. There are {} open windows.",
            NUM_WINDOWS.with(|n| n.get())
        );
    }

    // SAFETY: This cast must match the type of Box::into_raw().
    let window_state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;

    if window_state.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    } else {
        let r = wndproc(&*window_state, msg, wparam, lparam);

        if msg == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);

            let _ = Box::from_raw(window_state);

            NUM_WINDOWS.with(|n| n.set(n.get() - 1));

            tracing::debug!(
                "Window destroyed. There are {} open windows.",
                NUM_WINDOWS.with(|n| n.get())
            );

            if NUM_WINDOWS.with(|n| n.get() == 0) {
                tracing::debug!("All windows closed, exiting event loop.");
                PostQuitMessage(0);
            }
        }

        r
    }
}

fn wndproc(window: &WindowState, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let ret = match msg {
        WM_CREATE => {
            let handle = WindowHandle {
                inner: Rc::downgrade(&window.hwnd),
            };
            Some((0, Some(Event::Create(handle))))
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(window.hwnd.get()) };
            Some((0, None))
        }
        WM_DESTROY => Some((0, Some(Event::Destroy))),
        WM_CLOSE => Some((0, Some(Event::Close))),
        WM_ERASEBKGND => Some((1, None)),
        WM_WINDOWPOSCHANGED => Some((0, None)), // Swallows WM_SIZE
        WM_ENTERSIZEMOVE => Some((0, Some(Event::ResizeBegin))),
        WM_EXITSIZEMOVE => Some((0, Some(Event::ResizeEnd))),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(window.hwnd.get(), &mut ps) };
            unsafe { EndPaint(window.hwnd.get(), &ps) };

            let content_size = {
                let mut client_rect = RECT::default();
                unsafe {
                    GetClientRect(window.hwnd.get(), &mut client_rect);
                }
                Size2D::new(client_rect.right, client_rect.bottom)
                    .try_cast::<u16>()
                    .expect("Window size is negative or larger than u16::MAX")
            };

            Some((0, Some(Event::Paint(content_size))))
        }
        _ => None,
    };

    if let Some((ret, event)) = ret {
        if let Some(event) = event {
            EVENT_SINK.with(|s| s.borrow_mut().as_mut().unwrap().send(window.id, event));
        }

        return LRESULT(ret);
    } else {
        unsafe { DefWindowProcW(window.hwnd.get(), msg, wparam, lparam) }
    }
}
