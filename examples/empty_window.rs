use plinth::{
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    input::{Axis, ButtonState, MouseButton},
    math::{Point, Scale, Size, Vec2},
    Application, Window, WindowEventHandler, WindowSpec,
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

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        canvas.clear(Color::GREEN);
        println!("Window repaint requested for {:?}", timing.next_present);
    }

    fn on_mouse_button(
        &mut self,
        button: MouseButton,
        state: ButtonState,
        _location: Point<Window>,
    ) {
        println!("{:?} mouse button {:?}", button, state);
    }

    fn on_pointer_move(&mut self, location: Point<Window>, delta: Vec2<Window>) {
        println!("pointer at {:?} with delta {:?}", location, delta);
    }

    fn on_pointer_leave(&mut self) {
        println!("pointer left window",);
    }

    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        println!("scroll {:?} by {}", axis, delta);
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig::default());

    app.spawn_window(WindowSpec::default(), AppWindow::new);
    app.spawn_window(WindowSpec::default(), AppWindow::new);

    app.run();
}
