use plinth::{
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    system::{Window, WindowAttributes},
    time::{FramesPerSecond, PresentTime},
    AppContext, Application, Config, EventHandler,
};

const STARTING_REFRESH_RATE: FramesPerSecond = FramesPerSecond::new(60.0);

// // consume 100ms per frame (10fps), the clock should correct accordingly
// // const SLEEP_PER_FRAME: Duration = Duration::from_millis(100);

pub fn main() {
    let config = Config {
        graphics: GraphicsConfig {
            debug_mode: false,
            ..Default::default()
        },
        ..Default::default()
    };

    Application::new(&config).unwrap().run(App {}).unwrap();
}

pub struct AppWindow {
    refresh_rate: FramesPerSecond,
    prev_draw_start_time: PresentTime,
}

pub struct App {}

impl EventHandler<AppWindow> for App {
    fn start(&mut self, app: &mut AppContext<AppWindow>) {
        app.create_window(WindowAttributes::default(), |_| AppWindow {
            refresh_rate: STARTING_REFRESH_RATE,
            prev_draw_start_time: PresentTime::now(),
        })
        .unwrap();
    }

    fn stop(&mut self, _app: &mut AppContext<AppWindow>) {
        // no-op
    }

    fn window_wake_requested(
        &mut self,
        _app: &mut AppContext<AppWindow>,
        _window: &mut Window<AppWindow>,
    ) {
        // no-op
    }

    fn window_destroyed(&mut self, _app: &mut AppContext<AppWindow>, _window_data: AppWindow) {
        // no-op
    }

    fn window_frame(
        &mut self,
        _app: &mut AppContext<AppWindow>,
        window: &mut Window<AppWindow>,
        canvas: &mut Canvas,
        timing: &FrameInfo,
    ) {
        let this = window.data_mut();

        let now = PresentTime::now();
        let elapsed = now - this.prev_draw_start_time;
        this.prev_draw_start_time = now;

        canvas.clear(Color::BLUE);

        let instantaneous_frame_rate = FramesPerSecond::from_period(elapsed);

        println!(
                "repaint:\n    prev present time: {:?}\n    present time: {:?}\n    frame budget: {:?}\n    target refresh rate: {:?}\n    provided refresh rate: {:?}\n    estimated refresh rate: {:?}",
                timing.prev_present_time,
                timing.next_present_time,
                timing.next_present_time - timing.prev_present_time,
                this.refresh_rate,
                timing.target_frame_rate,
                instantaneous_frame_rate,
            );
    }
}
