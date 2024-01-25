use plinth::{
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    math::Point,
    Application, EventHandler, PhysicalPixel, Window, WindowSpec,
};

pub struct AppWindow {
    window: Window,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self { window }
    }
}

impl EventHandler for AppWindow {
    fn on_close_request(&mut self) {
        println!("Close request");
        self.window.close();
    }

    fn on_visible(&mut self, is_visible: bool) {
        println!("Visible: {}", is_visible);
    }

    fn on_begin_resize(&mut self) {
        println!("Begin resize");
    }

    fn on_end_resize(&mut self) {
        println!("End resize");
    }

    fn on_pointer_leave(&mut self) {
        println!("Pointer leave");
    }

    fn on_mouse_button(
        &mut self,
        button: plinth::MouseButton,
        state: plinth::ButtonState,
        location: Point<i16, PhysicalPixel>,
    ) {
        println!("Mouse button {:?} {:?} at {:?}", button, state, location);
    }

    fn on_pointer_move(&mut self, location: Point<i16, PhysicalPixel>) {
        println!("Pointer move to {:?}", location);
    }

    fn on_scroll(&mut self, axis: plinth::Axis, delta: f32) {
        println!("Scroll {:?} by {}", axis, delta);
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        println!("Repaint at {:?}", timing);
        canvas.clear(Color::GREEN);
    }
}

impl Drop for AppWindow {
    fn drop(&mut self) {
        println!("Destroy");
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig::default());

    app.spawn_window(WindowSpec::default(), AppWindow::new)
        .unwrap();

    app.run();
}
