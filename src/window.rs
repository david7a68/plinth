use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::OnceLock,
};

use arrayvec::ArrayVec;
use euclid::{Point2D, Size2D};
use smallvec::SmallVec;
use windows::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{
            GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WAIT_FAILED, WAIT_OBJECT_0,
            WIN32_ERROR, WPARAM,
        },
        Graphics::{
            DirectComposition::DCompositionWaitForCompositorClock,
            Gdi::{BeginPaint, EndPaint, InvalidateRect, HBRUSH, PAINTSTRUCT},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW, PeekMessageW, PostMessageW,
            PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, ShowWindow, ShowWindowAsync,
            TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA,
            HICON, HMENU, IDC_ARROW, MSG, PM_NOREMOVE, PM_REMOVE, SW_SHOW, WM_CLOSE, WM_CREATE,
            WM_DESTROY, WM_ENTERSIZEMOVE, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_PAINT,
            WM_QUIT, WM_TIMER, WM_USER, WM_WINDOWPOSCHANGED, WNDCLASSEXW,
            WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::graphics::{self, ResizeOp, Swapchain};

pub const MAX_TITLE_LENGTH: usize = 256;

/// Represents measurement units in pixels before any DPI scaling is applied.
pub struct ScreenSpace;

const CLASS_NAME: PCWSTR = w!("plinth_window_class");

/// Determines how quickly swapchain buffers grow when the window is resized.
/// This helps to amortize the cost of calling `ResizeBuffers` on every frame.
/// Once resized, the swapchain is set to the size of the window to conserve
/// memory.
const SWAPCHAIN_GROWTH_FACTOR: f32 = 1.2;

const UM_DESTROY_WINDOW: u32 = WM_USER;

// Global state
static WND_CLASS_ATOM: OnceLock<u16> = OnceLock::new(); // TODO: make thread local?

fn wnd_class_atom_as_pcwstr() -> PCWSTR {
    PCWSTR(WND_CLASS_ATOM.get().unwrap().clone() as usize as *const _)
}

thread_local! {
    static NUM_WINDOWS: Cell<usize> = const { Cell::new(0) };
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

/// Interface for per-window event handling.
///
/// The design of this interface takes a reference to a 'WindowControl' object
/// in order to maintain the unitary nature of the window object (as opposed to
/// `Clone`able handles). This reduces the likelihood of confusing one handle
/// for another, low as it might be, and gives the implementation a little bit
/// more flexibility.
///
/// ### Implementation Notes:
///
/// Model                          | Dangling Handles? | Dispatch Mechanism                         | Per-Window State
/// -------------------------------|-------------------|--------------------------------------------|-----------------
/// `Clone`able handles            | Yes               | Dynamic (`WindowHandler`)                  | `Rc<RefCell<>>`
/// Global event handler           | No                | Static (`match Event`)                     | `HashMap<WindowHandle, T>`
/// `trait WindowControl`          | No                | Dynamic (`WindowHandler`, `WindowControl`) | `impl WindowHandler`
/// `struct WindowControl` (this)  | No                | Dynamic (`WindowHandler`)                  | `impl WindowHandler`
pub trait WindowHandler {
    fn on_create(&mut self, window: WindowHandle);

    fn on_destroy(&mut self);

    fn on_close(&mut self);

    fn on_show(&mut self);

    fn on_hide(&mut self);

    fn on_move(&mut self, position: Point2D<i32, ScreenSpace>);

    fn on_resize(&mut self, size: Size2D<u16, ScreenSpace>);
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
    pub fn build(
        &self,
        graphics: Rc<RefCell<graphics::Device>>,
        event_handler: Box<dyn WindowHandler>,
    ) -> WindowHandle {
        register_window_class_once();

        let title = translate_title(self.title);

        let handle = Rc::default();
        let handle_weak = Rc::downgrade(&handle);

        let inner_state = Box::new(WindowState {
            hwnd: handle,
            content_size: Cell::default(),
            is_resizing: Cell::new(false),
            graphics,
            swapchain: RefCell::new(None),
            swapchain_size: Cell::default(),
            event_handler: RefCell::new(event_handler),
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
    hwnd: Rc<Cell<HWND>>,

    /// The size of the content area (excluding borders and title bar).
    content_size: Cell<Size2D<u16, ScreenSpace>>,

    is_resizing: Cell<bool>,

    graphics: Rc<RefCell<graphics::Device>>,
    swapchain: RefCell<Option<Swapchain>>,

    /// The size of the swapchain (may be larger than the window size).
    swapchain_size: Cell<Size2D<u16, ScreenSpace>>,

    event_handler: RefCell<Box<dyn WindowHandler>>,
}

pub struct EventLoop {}

impl EventLoop {
    /// Hijacks the current thread to run the event loop. The thread will
    /// terminate once all windows are destroyed.
    ///
    /// All windows must be created on the same thread that runs the event loop,
    /// or they will not receive events.
    pub fn run() {
        loop {
            let mut msg = MSG::default();

            // Force any pending timer messages to be generated. This is in case
            // the message queue keeps getting higher priority messages faster
            // than it can process them.
            unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

            // if all windows are closed, exit the event loop

            if true {
                // Continuous mode. This triggers on every tick of the
                // compositor clock. Useful for animations and media playback.

                // Process all pending messages.
                while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                    if msg.message == WM_QUIT {
                        return;
                    }

                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                // handle scheduled presentation

                let wait_result = unsafe { DCompositionWaitForCompositorClock(None, u32::MAX) };

                match WIN32_ERROR(wait_result) {
                    WAIT_OBJECT_0 => {
                        // the compositor clock has ticked
                        continue;
                    }
                    WAIT_FAILED => {
                        panic!(
                            "Failed to wait for compositor clock, error code: {}",
                            unsafe { GetLastError() }.0
                        );
                    }
                    _ => {
                        break;
                    }
                }
            } else {
                // Input mode. This only triggers on events like user input or
                // scheduled timers. Useful for conserving power.

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
        }
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

        // SAFETY: This cast must match the type of Box::into_raw().
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

            let ws = Box::from_raw(window_state);
            ws.graphics.borrow_mut().flush();
            std::mem::drop(ws);

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
            let swapchain = window.graphics.borrow().create_swapchain(window.hwnd.get());
            let _ = window.swapchain.replace(Some(swapchain));

            let content_size = {
                let mut client_rect = RECT::default();
                unsafe {
                    GetClientRect(window.hwnd.get(), &mut client_rect);
                }
                Size2D::new(client_rect.right, client_rect.bottom)
                    .try_cast::<u16>()
                    .expect("Window size is negative or larger than u16::MAX")
            };

            window.content_size.set(content_size);
            window.swapchain_size.set(content_size);

            let handle = WindowHandle {
                inner: Rc::downgrade(&window.hwnd),
            };

            window.event_handler.borrow_mut().on_create(handle);
            Some(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(window.hwnd.get()) };
            Some(0)
        }
        WM_DESTROY => {
            window.event_handler.borrow_mut().on_destroy();
            Some(0)
        }
        WM_CLOSE => {
            window.event_handler.borrow_mut().on_close();
            Some(0)
        }
        WM_ERASEBKGND => Some(1),
        WM_WINDOWPOSCHANGED => {
            // Handling this means we don't get a WM_SIZE message
            Some(0)
        }
        WM_ENTERSIZEMOVE => {
            tracing::debug!("WM_ENTERSIZEMOVE");
            // increment anim_request_count

            window.is_resizing.set(true);

            Some(0)
        }
        WM_EXITSIZEMOVE => {
            tracing::debug!("WM_EXITSIZEMOVE");
            // decrement anim_request_count

            window.is_resizing.set(false);

            if window.content_size.get() != window.swapchain_size.get() {
                tracing::debug!("boo!");
                let size = window.content_size.get();

                let mut swp = window.swapchain.borrow_mut();
                let swapchain = swp.as_mut().unwrap();

                swapchain.resize(ResizeOp::Auto);
                window.swapchain_size.set(size);

                unsafe { InvalidateRect(window.hwnd.get(), None, false) };
            }

            Some(0)
        }
        WM_PAINT => {
            tracing::trace!("WM_PAINT");

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

            let mut swp = window.swapchain.borrow_mut();
            let swapchain = swp.as_mut().unwrap();
            let mut graphics = window.graphics.borrow_mut();

            if content_size != window.content_size.get() {
                tracing::debug!("resizing to {}x{}", content_size.width, content_size.height);

                graphics.flush();

                if window.is_resizing.get() {
                    swapchain.resize(ResizeOp::Flex {
                        size: content_size,
                        flex: SWAPCHAIN_GROWTH_FACTOR,
                    });
                } else {
                    swapchain.resize(ResizeOp::Auto);
                }

                window.content_size.set(content_size);

                window.event_handler.borrow_mut().on_resize(content_size);
            }

            let (target, _target_idx) = swapchain.get_back_buffer();

            let canvas = graphics.create_canvas(target);
            // window.event_handler.borrow_mut().on_paint(&mut canvas);
            graphics.draw_canvas(canvas);
            swapchain.present();

            Some(0)
        }
        _ => None,
    };

    if let Some(ret) = ret {
        return LRESULT(ret);
    } else {
        unsafe { DefWindowProcW(window.hwnd.get(), msg, wparam, lparam) }
    }
}
