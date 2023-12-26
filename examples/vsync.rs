use plinth::{
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    input::Axis,
    time::FramesPerSecond,
    Application, Window, WindowEventHandler, WindowSpec,
};

#[cfg(feature = "profile")]
use tracing_subscriber::layer::SubscriberExt;

const STARTING_REFRESH_RATE: FramesPerSecond = FramesPerSecond(60.0);

// consume 100ms per frame (10fps), the clock should correct accordingly
// const SLEEP_PER_FRAME: Duration = Duration::from_millis(100);

pub struct AppWindow {
    window: Window,
    refresh_rate: FramesPerSecond,
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

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        // print frame stats: last frame's present time, frame budget, and current refresh rate
        println!(
            "repaint:\n    prev present time: {:?}\n    present time: {:?}\n    frame budget: {:?}\n    target refresh rate: {:?}\n    estimated refresh rate: {:?}",
            timing.prev_present_time,
            timing.next_present_time,
            timing.next_present_time - timing.prev_present_time,
            self.refresh_rate,
            timing.frame_rate,
        );
        canvas.clear(Color::RED);

        // std::thread::sleep(SLEEP_PER_FRAME);
    }

    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        if axis == Axis::Y {
            self.refresh_rate = (self.refresh_rate + delta as _).max(FramesPerSecond::ZERO);

            self.window.set_animation_frequency(self.refresh_rate);
        }
    }
}

pub fn main() {
    #[cfg(feature = "profile")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    #[cfg(not(feature = "profile"))]
    tracing_subscriber::fmt::fmt().pretty().init();

    let mut app = Application::new(&GraphicsConfig {
        debug_mode: true,
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
