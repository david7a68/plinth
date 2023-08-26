mod graphics;
mod window;

use std::{cell::RefCell, rc::Rc};

use crate::graphics::*;
use crate::window::*;
use euclid::{Point2D, Size2D};

struct AppWindow {}

impl WindowHandler for AppWindow {
    fn on_create(&mut self, window: &mut WindowControl) {
        window.show();
    }

    fn on_destroy(&mut self, _window: &mut WindowControl) {
        // todo
    }

    fn on_close(&mut self, window: &mut WindowControl) {
        window.destroy();
    }

    fn on_show(&mut self, _window: &mut WindowControl) {
        // todo
    }

    fn on_hide(&mut self, _window: &mut WindowControl) {
        // todo
    }

    fn on_move(&mut self, _window: &mut WindowControl, _position: Point2D<i32, ScreenSpace>) {
        // todo
    }

    fn on_resize(&mut self, _window: &mut WindowControl, _size: Size2D<u16, ScreenSpace>) {
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
    .build(graphics.clone(), Box::new(AppWindow {}));

    // let _window2 = WindowSpec {
    //     title: "Isn't this nice?",
    //     size: Size2D::new(800, 600),
    // }
    // .build(renderer.clone(), Box::new(AppWindow {}));

    EventLoop::run();
}
