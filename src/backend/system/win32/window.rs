use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
    time::Instant,
};

use arrayvec::ArrayVec;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetClientRect, GetMessageW, LoadCursorW, PeekMessageW, PostMessageW, PostQuitMessage,
            RegisterClassExW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, HICON, HMENU, IDC_ARROW, MSG, PM_NOREMOVE, SW_HIDE, SW_SHOW, WM_CLOSE,
            WM_CREATE, WM_DESTROY, WM_EXITSIZEMOVE, WM_PAINT, WM_SHOWWINDOW, WM_SIZING, WM_TIMER,
            WM_USER, WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::{
    animation::{AnimationFrequency, PresentTiming},
    application::AppContext,
    math::{Point, Scale, Size},
    visuals::Pixel,
    window::{WindowEvent, WindowEventHandler, WindowSpec, MAX_TITLE_LENGTH},
};

use super::AppMessage;

const CLASS_NAME: PCWSTR = w!("plinth_window_class");
const UM_DESTROY_WINDOW: u32 = WM_USER;

thread_local! {
    static STATE: State = State::default();
    static EVENT_HANDLER: RefCell<Option<Box<dyn WindowEventHandler>>> = RefCell::new(None);
}

#[derive(Debug, Default)]
pub struct State {
    initialized: Cell<bool>,
    is_resizing: Cell<bool>,
    size: Cell<Size<crate::window::Window>>,
}

pub struct Window {
    hwnd: HWND,
    context: AppContext,
}

impl Window {
    pub fn app(&self) -> &AppContext {
        &self.context
    }

    pub fn close(&mut self) {
        unsafe { PostMessageW(self.hwnd, UM_DESTROY_WINDOW, None, None) }.unwrap();
    }

    pub fn begin_animation(&mut self, _freq: Option<AnimationFrequency>) {
        todo!()
    }

    pub fn end_animation(&mut self) {
        todo!()
    }

    pub fn default_animation_frequency(&self) -> AnimationFrequency {
        todo!()
    }

    pub fn size(&self) -> Size<crate::window::Window> {
        todo!()
    }

    pub fn scale(&self) -> Scale<crate::window::Window, Pixel> {
        todo!()
    }

    pub fn set_visible(&mut self, visible: bool) {
        let flag = if visible { SW_SHOW } else { SW_HIDE };
        unsafe { ShowWindow(self.hwnd, flag) };
    }

    pub fn pointer_location(&self) -> Point<crate::window::Window> {
        todo!()
    }
}

pub fn spawn_window_thread<W, F>(context: AppContext, spec: WindowSpec, mut constructor: F)
where
    W: WindowEventHandler + 'static,
    F: FnMut(crate::window::Window) -> W + Send + 'static,
{
    std::thread::spawn(move || {
        let wndclass = ensure_wndclass_registered();

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

        let mut constructor = |window| Box::new(constructor(window)) as Box<dyn WindowEventHandler>;
        let create_info = RefCell::new(CreateInfo {
            context: Some(context.clone()),
            constructor: &mut constructor,
        });

        let hwnd = unsafe {
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
                Some(&create_info as *const RefCell<_> as _),
            )
        };

        if spec.visible {
            unsafe { ShowWindow(hwnd, SW_SHOW) };
        }

        context
            .inner
            .sender
            .send(AppMessage::WindowCreated)
            .unwrap();

        let mut msg = MSG::default();
        loop {
            // Force any pending timer messages to be generated. This is in case
            // the message queue keeps getting higher priority messages faster
            // than it can process them.
            unsafe { PeekMessageW(&mut msg, None, WM_TIMER, WM_TIMER, PM_NOREMOVE) };

            // TODO: Make this more sophisticated.
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

            // if redraw requested
            //     redraw

            // if animating
            //    wait for next vsync
        }

        context.inner.sender.send(AppMessage::WindowClosed).unwrap();
    });
}

struct CreateInfo<'a> {
    context: Option<AppContext>,
    constructor: &'a mut dyn FnMut(crate::window::Window) -> Box<dyn WindowEventHandler>,
}

fn ensure_wndclass_registered() -> PCWSTR {
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
    PCWSTR(WND_CLASS_ATOM.get().unwrap().clone() as usize as *const _)
}

unsafe extern "system" fn wndproc_trampoline(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = lparam.0 as *const CREATESTRUCTW;
        let create_info = &*((*create_struct).lpCreateParams as *const RefCell<CreateInfo>);
        let mut ci = create_info.borrow_mut();

        let window = crate::window::Window::new(Window {
            hwnd,
            context: ci.context.take().unwrap(),
        });

        let handler = (ci.constructor)(window);
        EVENT_HANDLER.set(Some(handler));
        STATE.with(|s| s.initialized.set(true));
    }

    STATE.with(|state| {
        if state.initialized.get() {
            wndproc(state, hwnd, msg, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    })
}

fn wndproc(state: &State, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            dispatch_event(WindowEvent::CloseRequest);
            LRESULT(0)
        }
        UM_DESTROY_WINDOW => {
            unsafe { DestroyWindow(hwnd) }.unwrap();
            LRESULT(0)
        }
        WM_DESTROY => {
            dispatch_event(WindowEvent::Destroy);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            dispatch_event(WindowEvent::Visible(wparam.0 != 0));
            LRESULT(0)
        }
        WM_SIZING => {
            state.is_resizing.set(true);
            dispatch_event(WindowEvent::BeginResize);
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            if state.is_resizing.get() {
                state.is_resizing.set(false);
                dispatch_event(WindowEvent::EndResize);
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            let size = unsafe {
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect).unwrap();

                Size::new(rect.right as f64, rect.bottom as f64)
            };

            // we don't care about window position, so ignore it

            if size != state.size.get() {
                state.size.set(size);
                dispatch_event(WindowEvent::Resize(size, Scale::default()));
            }

            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            unsafe { EndPaint(hwnd, &ps) };

            // TODO: This should be somewhere else, unless during resize
            dispatch_event(WindowEvent::Repaint(PresentTiming {
                next_frame: Instant::now(),
                last_frame: Instant::now(),
            }));

            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn dispatch_event(event: WindowEvent) {
    EVENT_HANDLER.with_borrow_mut(|handler| handler.as_mut().unwrap().event(event));
}
