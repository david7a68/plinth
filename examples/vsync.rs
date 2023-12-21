use std::time::Duration;

use plinth::{
    application::Application,
    graphics::{Canvas, Color, FrameStatistics, GraphicsConfig, PresentDuration},
    input::Axis,
    window::{Window, WindowEventHandler, WindowSpec},
};

const STARTING_REFRESH_RATE: f32 = 60.0;

// consume 100ms per frame (10fps), the clock should correct accordingly
const SLEEP_PER_FRAME: Duration = Duration::from_millis(100);

pub struct AppWindow {
    window: Window,
    refresh_rate: f32,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self {
            window,
            refresh_rate: STARTING_REFRESH_RATE,
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
            "repaint:\n    prev present time: {:?}\n    present time: {:?}\n    frame budget: {:?}\n    target refresh rate: {}\n    estimated refresh rate: {}",
            timing.prev_present_time,
            timing.next_present_time,
            timing.next_present_time - timing.prev_present_time,
            self.refresh_rate,
            PresentDuration::from_secs_f32(1.0) / (timing.next_present_time - timing.prev_present_time)
        );
        canvas.clear(Color::RED);

        // std::thread::sleep(SLEEP_PER_FRAME);
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
            refresh_rate: Some(STARTING_REFRESH_RATE),
            ..Default::default()
        },
        AppWindow::new,
    );

    app.run();
}
