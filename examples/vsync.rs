use std::sync::Arc;

use plinth::{
    application::Application,
    graphics::{Canvas, Color, FrameStatistics, GraphicsConfig},
    input::Axis,
    window::{Window, WindowEventHandler, WindowSpec},
};

pub struct AppWindow {
    window: Window,
    refresh_rate: f32,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self {
            window,
            refresh_rate: 60.0,
        }
    }
}

impl WindowEventHandler for AppWindow {
    fn on_close_request(&mut self) {
        self.window.close();
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameStatistics) {
        // print frame stats: last frame's present time, frame budget, and current refresh rate
        println!(
            "repaint:\n    prev present time: {:?}\n    present time: {:?}\n    frame budget: {:?}\n    target refresh rate: {}",
            timing.prev_present_time,
            timing.next_present_time,
            timing.next_present_time - timing.prev_present_time,
            self.refresh_rate,
        );
        canvas.clear(Color::RED);
    }

    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        if axis == Axis::Y {
            self.refresh_rate = 0.0_f32.max(self.refresh_rate + delta);
            self.window.set_animation_frequency(self.refresh_rate);
        }
    }
}

pub fn main() {
    tracing_subscriber::fmt().pretty().init();

    let mut app = Application::new(&GraphicsConfig {
        debug_mode: false,
        ..Default::default()
    });

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
