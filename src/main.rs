mod graphics;
mod shell;
mod window;

use euclid::Size2D;

#[cfg(feature = "profile")]
use tracing_tracy::client::{plot, span_location};

#[derive(Default)]
struct Window {
    handle: Option<shell::WindowHandle>,
}

impl Window {
    pub fn new() -> Self {
        Self::default()
    }
}

impl shell::WindowEventHandler for Window {
    fn on_event(&mut self, event: shell::WindowEvent) {
        match event {
            shell::WindowEvent::Create(handle) => self.handle = Some(handle),
            shell::WindowEvent::CloseRequest => self.handle.as_ref().unwrap().destroy().unwrap(),
            shell::WindowEvent::Destroy => {
                tracing::info!("Window destroyed");
            }
            shell::WindowEvent::Resize(size) => {
                tracing::info!("Window resized to {:?}", size);
            }
            shell::WindowEvent::Repaint => {
                tracing::info!("Window repainted");
            }
        }
    }
}

fn main() {
    #[cfg(feature = "profile")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
        )
        .expect("set up the subscriber");
    }

    #[cfg(not(feature = "profile"))]
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let event_loop = shell::EventLoop::new();

    let window = shell::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_content_size(Size2D::new(800, 600))
        .with_event_handler(Window::new())
        .build();

    event_loop.run();
}
