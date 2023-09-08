mod graphics;
mod shell;

use euclid::Size2D;

use graphics::{
    thread::{RenderThread, RenderThreadProxy, WindowId},
    GraphicsConfig, ResizeOp,
};
use shell::{WindowEventHandler, WindowHandle};
#[cfg(feature = "profile")]
use tracing_tracy::client::{plot, span_location};

struct AppWindow {
    state: AppWindowState,
    render_proxy: RenderThreadProxy,
    first_paint: bool,
    is_resizing: bool,
}

enum AppWindowState {
    New,
    Usable { id: WindowId, handle: WindowHandle },
}

impl AppWindow {
    fn new(render_proxy: RenderThreadProxy) -> Self {
        Self {
            state: AppWindowState::New,
            render_proxy,
            first_paint: true,
            is_resizing: false,
        }
    }
}

impl WindowEventHandler for AppWindow {
    #[tracing::instrument(skip(self))]
    fn on_event(&mut self, event: shell::WindowEvent) {
        match event {
            shell::WindowEvent::Create(handle) => {
                self.state = AppWindowState::Usable {
                    id: self.render_proxy.new_window(handle.clone()),
                    handle,
                };
            }
            shell::WindowEvent::CloseRequest => {
                let AppWindowState::Usable { id: _, handle } = &self.state else {
                    panic!("Window close request on non-usable window")
                };

                handle.destroy().unwrap();
            }
            shell::WindowEvent::Destroy => {
                let AppWindowState::Usable { id, handle: _ } = &self.state else {
                    panic!("Window destroy on non-usable window")
                };

                self.render_proxy.destroy_window(*id);
            }
            shell::WindowEvent::BeginResize => {
                let AppWindowState::Usable { id, handle: _ } = &self.state else {
                    panic!("Window resize on non-usable window")
                };

                self.render_proxy.disable_vsync(*id);
                self.is_resizing = true;
            }
            shell::WindowEvent::Resize(size) => {
                let AppWindowState::Usable { id, handle: _ } = &self.state else {
                    panic!("Window resize on non-usable window")
                };

                let op = self
                    .is_resizing
                    .then_some(ResizeOp::Flex { size, flex: 1.2 })
                    .unwrap_or(ResizeOp::Auto);

                self.render_proxy.resize_window(*id, op);
            }
            shell::WindowEvent::EndResize => {
                let AppWindowState::Usable { id, handle: _ } = &self.state else {
                    panic!("Window resize on non-usable window")
                };

                self.render_proxy.resize_window(*id, ResizeOp::Auto);
                self.render_proxy.enable_vsync(*id);
                self.is_resizing = false;
            }
            shell::WindowEvent::Repaint => {
                let AppWindowState::Usable { id, handle: _ } = &self.state else {
                    panic!("Window repaint on non-usable window")
                };

                if self.first_paint {
                    self.first_paint = false;
                    self.render_proxy.enable_vsync(*id);
                } else if self.is_resizing {
                    self.render_proxy.force_draw(*id);

                    // cannot invalidate window here, else we end up in an infinite loop
                }
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

    let (_render_thread, render_proxy) = RenderThread::spawn(GraphicsConfig { debug_mode: false });

    let event_loop = shell::EventLoop::new();

    let _window = shell::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_content_size(Size2D::new(800, 600))
        .with_event_handler(AppWindow::new(render_proxy.clone()))
        .build();

    let _window = shell::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_content_size(Size2D::new(800, 600))
        .with_event_handler(AppWindow::new(render_proxy.clone()))
        .build();

    let _window = shell::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_content_size(Size2D::new(800, 600))
        .with_event_handler(AppWindow::new(render_proxy.clone()))
        .build();

    let _window = shell::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_content_size(Size2D::new(800, 600))
        .with_event_handler(AppWindow::new(render_proxy))
        .build();

    event_loop.run();
}
