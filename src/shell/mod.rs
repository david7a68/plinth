mod win32;

use std::cell::RefCell;

use euclid::Size2D;
use windows::Win32::Foundation::HWND;

pub const MAX_TITLE_LENGTH: usize = 256;

/// Represents measurement units in pixels before any DPI scaling is applied.
pub struct ScreenSpace;

pub enum WindowEvent {
    Create(WindowHandle),
    CloseRequest,
    Destroy,
    // Show,
    // Hide,
    BeginResize,
    Resize(Size2D<u16, ScreenSpace>),
    EndResize,
    Repaint,
}

impl std::fmt::Debug for WindowEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create(_) => f.debug_tuple("Create").finish(),
            Self::CloseRequest => write!(f, "CloseRequest"),
            Self::Destroy => write!(f, "Destroy"),
            Self::BeginResize => write!(f, "BeginResize"),
            Self::Resize(arg0) => f.debug_tuple("Resize").field(arg0).finish(),
            Self::EndResize => write!(f, "EndResize"),
            Self::Repaint => write!(f, "Repaint"),
        }
    }
}

pub trait WindowEventHandler {
    fn on_event(&mut self, event: WindowEvent);
}

// Todo: Does this cause double indirection?
impl<F: FnMut(WindowEvent) + 'static> WindowEventHandler for F {
    fn on_event(&mut self, event: WindowEvent) {
        (self)(event);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowError {
    AlreadyDestroyed,
}

#[derive(Default)]
pub struct WindowSpec {
    title: String,
    size: Size2D<u16, ScreenSpace>,
}

#[derive(Default)]
pub struct WindowBuilder {
    spec: WindowSpec,
    event_handler: Option<Box<dyn WindowEventHandler>>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.spec.title = title.into();
        self
    }

    pub fn with_content_size(mut self, size: Size2D<u16, ScreenSpace>) -> Self {
        self.spec.size = size;
        self
    }

    pub fn with_event_handler<Handler: WindowEventHandler + 'static>(
        mut self,
        handler: Handler,
    ) -> Self {
        self.event_handler = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> WindowHandle {
        WindowHandle {
            handle: win32::build_window(
                self.spec,
                WindowState::new(
                    self.event_handler
                        .expect("A window must have an event handler."),
                ),
            ),
        }
    }
}

#[derive(Clone)]
pub struct WindowHandle {
    handle: win32::WindowHandle,
}

impl WindowHandle {
    pub fn hwnd(&self) -> Result<HWND, WindowError> {
        self.handle.hwnd()
    }

    pub fn content_size(&self) -> Result<Size2D<u16, ScreenSpace>, WindowError> {
        self.handle.content_size()
    }

    pub fn show(&self) -> Result<(), WindowError> {
        self.handle.show()
    }

    pub fn destroy(&self) -> Result<(), WindowError> {
        self.handle.destroy()
    }

    pub fn request_redraw(&self) -> Result<(), WindowError> {
        self.handle.request_redraw()
    }
}

pub struct EventLoop {
    event_loop: win32::EventLoop,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            event_loop: win32::EventLoop::new(),
        }
    }

    pub fn run(&self) {
        self.event_loop.run();
    }
}

struct WindowState {
    event_handler: RefCell<Box<dyn WindowEventHandler>>,
    // input events should be batched together, but resize events need to be handled synchronously...
}

impl WindowState {
    fn new(event_handler: Box<dyn WindowEventHandler>) -> Self {
        Self {
            event_handler: RefCell::new(event_handler),
        }
    }

    fn on_create(&self, hande: WindowHandle) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::Create(hande));
    }

    fn on_close_request(&self) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::CloseRequest);
    }

    fn on_destroy(&self) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::Destroy);
    }

    fn on_resize_begin(&self) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::BeginResize);
    }

    fn on_resize(&self, size: Size2D<u16, ScreenSpace>) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::Resize(size));
    }

    fn on_resize_end(&self) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::EndResize);
    }

    fn on_paint(&self) {
        self.event_handler
            .borrow_mut()
            .on_event(WindowEvent::Repaint);
    }
}
