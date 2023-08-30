mod graphics;
mod window;

use std::{cell::RefCell, rc::Rc};

use crate::graphics::*;
use crate::window::*;
use euclid::{Point2D, Size2D};

struct AppWindow {
    handle: Option<WindowHandle>,
}

impl AppWindow {
    fn new() -> Self {
        Self { handle: None }
    }
}

impl WindowHandler for AppWindow {
    fn on_create(&mut self, window: WindowHandle) {
        window.show();
        self.handle = Some(window);
    }

    fn on_destroy(&mut self) {
        // todo
    }

    fn on_close(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.destroy();
        }
    }

    fn on_show(&mut self) {
        // todo
    }

    fn on_hide(&mut self) {
        // todo
    }

    fn on_move(&mut self, _position: Point2D<i32, ScreenSpace>) {
        // todo
    }

    fn on_resize(&mut self, _size: Size2D<u16, ScreenSpace>) {
        // todo
    }
}

fn main() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let graphics_config = GraphicsConfig { debug_mode: true };

    let graphics = Rc::new(RefCell::new(graphics::Device::new(&graphics_config)));

    let _window1 = WindowSpec {
        title: "Oh look, windows!",
        size: Size2D::new(800, 600),
    }
    .build(graphics.clone(), Box::new(AppWindow::new()));

    // let _window2 = WindowSpec {
    //     title: "Isn't this nice?",
    //     size: Size2D::new(800, 600),
    // }
    // .build(renderer.clone(), Box::new(AppWindow {}));

    EventLoop::run();
}
