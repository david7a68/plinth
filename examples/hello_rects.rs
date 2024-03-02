use plinth::{
    geometry::pixel::Rect,
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig, RoundRect},
    system::window::{Window, WindowAttributes},
    AppContext, Application, EventHandler,
};

fn main() {
    #[cfg(feature = "profile")]
    {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
        )
        .expect("set up the subscriber");

        tracing_tracy::client::set_thread_name!("Main Thread");
    }

    #[cfg(not(feature = "profile"))]
    tracing_subscriber::fmt::fmt().pretty().init();

    let graphics_config = GraphicsConfig {
        debug_mode: false,
        ..Default::default()
    };

    Application::new(&graphics_config)
        .unwrap()
        .run(App {})
        .unwrap();
}

pub struct AppWindow {}

pub struct App {}

impl EventHandler<AppWindow> for App {
    fn start(&mut self, app: &AppContext<AppWindow>) {
        app.create_window(WindowAttributes::default(), |_| AppWindow {})
            .unwrap();

        app.create_window(WindowAttributes::default(), |_| AppWindow {})
            .unwrap();
    }

    fn stop(&mut self) {
        // no-op
    }

    fn wake_requested(&mut self, _app: &AppContext<AppWindow>, _window: &mut Window<AppWindow>) {
        // no-op
    }

    fn destroyed(&mut self, _app: &AppContext<AppWindow>, _window_data: AppWindow) {
        // no-op
    }

    fn repaint(
        &mut self,
        _app: &AppContext<AppWindow>,
        _window: &mut Window<AppWindow>,
        canvas: &mut Canvas,
        _frame: &FrameInfo,
    ) {
        canvas.clear(Color::BLACK);
        canvas
            .draw_rect(RoundRect::new(Rect::new(50.0, 100.0, 40.0, 70.0)).with_color(Color::BLUE));
        canvas
            .draw_rect(RoundRect::new(Rect::new(100.0, 100.0, 40.0, 70.0)).with_color(Color::RED));
    }
}
