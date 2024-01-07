use plinth::{
    frame::{FramesPerSecond, RedrawRequest},
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig, RoundRect},
    math::Rect,
    Application, Window, WindowEvent, WindowEventHandler, WindowSpec,
};

#[cfg(feature = "profile")]
use tracing_subscriber::layer::SubscriberExt;

struct DemoWindow {
    window: Window,
}

impl DemoWindow {
    fn new(mut window: Window) -> Self {
        window.request_redraw(RedrawRequest::AtFrameRate(FramesPerSecond(60.0)));
        Self { window }
    }
}

impl WindowEventHandler for DemoWindow {
    fn on_event(&mut self, event: plinth::WindowEvent) {
        match event {
            WindowEvent::CloseRequest => self.window.close(),
            _ => {}
        }
    }

    fn on_input(&mut self, _input: plinth::Input) {
        // no-op
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, _timing: &FrameInfo) {
        canvas.clear(Color::BLACK);
        canvas.draw_rect(RoundRect::builder(Rect::new(50.0, 100.0, 40.0, 70.0)).color(Color::BLUE));
        canvas.draw_rect(RoundRect::builder(Rect::new(100.0, 100.0, 40.0, 70.0)).color(Color::RED));

        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}

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

    let mut app = Application::new(&GraphicsConfig {
        debug_mode: false,
        ..Default::default()
    });

    let spec = WindowSpec::default();
    app.spawn_window(spec.clone(), |window| Box::new(DemoWindow::new(window)))
        .unwrap();
    app.spawn_window(spec.clone(), |window| Box::new(DemoWindow::new(window)))
        .unwrap();
    // app.spawn_window(spec, |window| Box::new(DemoWindow::new(window)))
    //     .unwrap();
    app.run();
}
