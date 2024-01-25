use plinth::{
    frame::{FramesPerSecond, RedrawRequest, SecondsPerFrame},
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    math::{Point, Size},
    time::Instant,
    Application, Axis, EventHandler, PhysicalPixel, Window, WindowSpec,
};

#[cfg(feature = "profile")]
use tracing_subscriber::layer::SubscriberExt;

const STARTING_REFRESH_RATE: FramesPerSecond = FramesPerSecond(60.0);

// consume 100ms per frame (10fps), the clock should correct accordingly
// const SLEEP_PER_FRAME: Duration = Duration::from_millis(100);

pub struct AppWindow {
    window: Window,
    refresh_rate: FramesPerSecond,
    prev_draw_start_time: Instant,
}

impl AppWindow {
    fn new(mut window: Window) -> Self {
        window.request_redraw(RedrawRequest::AtFrameRate(STARTING_REFRESH_RATE));

        Self {
            window,
            refresh_rate: STARTING_REFRESH_RATE,
            prev_draw_start_time: Instant::now(),
        }
    }
}

impl EventHandler for AppWindow {
    fn on_close_request(&mut self) {
        self.window.close();
    }

    fn on_mouse_button(
        &mut self,
        _button: plinth::MouseButton,
        _state: plinth::ButtonState,
        _location: Point<i16, PhysicalPixel>,
    ) {
        // no-op
    }

    fn on_pointer_move(&mut self, _location: Point<i16, PhysicalPixel>) {
        // no-op
    }

    fn on_scroll(&mut self, axis: Axis, delta: f32) {
        if axis == Axis::Y {
            self.refresh_rate = (self.refresh_rate + delta as _).max(FramesPerSecond::ZERO);
            self.window
                .request_redraw(RedrawRequest::AtFrameRate(self.refresh_rate));
        }
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        let now = Instant::now();
        let elapsed = now - self.prev_draw_start_time;
        self.prev_draw_start_time = now;

        canvas.clear(Color::BLUE);

        let instantaneous_frame_rate = SecondsPerFrame(elapsed).as_frames_per_second();

        tracing::info!(
                "repaint:\n    prev present time: {:?}\n    present time: {:?}\n    frame budget: {:?}\n    target refresh rate: {:?}\n    provided refresh rate: {:?}\n    estimated refresh rate: {:?}",
                timing.prev_present_time,
                timing.next_present_time,
                timing.next_present_time - timing.prev_present_time,
                self.refresh_rate,
                timing.target_frame_rate,
                instantaneous_frame_rate,
            );
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
        debug_mode: false,
        ..Default::default()
    });

    app.spawn_window(
        WindowSpec {
            title: "VSync Demo".to_owned(),
            size: Size::new(640, 480),
            ..Default::default()
        },
        AppWindow::new,
    )
    .unwrap();

    app.run();
}
