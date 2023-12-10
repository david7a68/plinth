use std::time::Instant;

use plinth::{
    application::Application,
    graphics::{Canvas, Color, FrameStatistics, GraphicsConfig},
    input::Axis,
    window::{Window, WindowEventHandler, WindowSpec},
};

pub struct AppWindow {
    window: Window,
    prev_paint: Instant,
    refresh_rate: f32,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self {
            window,
            prev_paint: Instant::now(),
            refresh_rate: 60.0,
        }
    }
}

impl WindowEventHandler for AppWindow {
    fn on_close_request(&mut self) {
        self.window.close();
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameStatistics) {
        println!(
            "delta: {:?}",
            timing.next_estimated_present - self.prev_paint
        );
        canvas.clear(Color::RED);

        self.prev_paint = timing.next_estimated_present;
    }

    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        if axis == Axis::Y {
            println!("refresh rate: {}", self.refresh_rate);

            self.refresh_rate = 0.0_f32.max(self.refresh_rate + delta);
            self.window.set_animation_frequency(self.refresh_rate);
        }
    }
}

pub fn main() {
    let mut app = Application::new(&GraphicsConfig::default());

    app.spawn_window(
        WindowSpec {
            title: "VSync Demo".to_owned(),
            size: (640, 480).into(),
            refresh_rate: Some(60.0),
            ..Default::default()
        },
        AppWindow::new,
    );

    app.run();
}
