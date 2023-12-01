use plinth::{
    animation::PresentTiming,
    application::{Application, GraphicsConfig},
    math::{Point, Scale, Size, Vec2},
    window::{Axis, Window, WindowEventHandler, WindowSpec},
};

pub struct AppWindow {
    window: Window,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self { window }
    }
}

impl WindowEventHandler for AppWindow {
    fn on_close_request(&mut self) {
        println!("Window close requested");
        self.window.close();
    }

    fn on_destroy(&mut self) {
        println!("Window destroyed");
    }

    fn on_visible(&mut self, is_visible: bool) {
        if is_visible {
            println!("Window is visible");
        } else {
            println!("Window is hidden");
        }
    }

    fn on_begin_resize(&mut self) {
        println!("Window resize started");
    }

    fn on_resize(&mut self, size: Size<Window>, scale: Scale<Window, Window>) {
        println!("Window resized to {} at {:?}", size, scale);
    }

    fn on_end_resize(&mut self) {
        println!("Window resize ended");
    }

    fn on_repaint(&mut self, timing: PresentTiming) {
        println!("Window repaint requested for {:?}", timing.next_frame);
    }

    fn on_pointer_move(&mut self, _location: Point<Window>, _delta: Vec2<Window>) {
        todo!()
    }

    fn on_scroll(&mut self, _axis: Axis, _delta: f32) {
        todo!()
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig::default());

    app.spawn_window(WindowSpec::default(), AppWindow::new);
    app.spawn_window(WindowSpec::default(), AppWindow::new);

    app.run();
}
