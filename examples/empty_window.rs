use plinth::{
    application::{Application, GraphicsConfig, PowerPreference},
    window::{Window, WindowEvent, WindowEventHandler, WindowSpec},
};

pub struct AppWindow {
    window: Window,
}

impl AppWindow {
    fn new(window: Window) -> Self {
        Self { window }
    }
}

impl WindowEventHandler for AppWindow {
    fn event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequest => self.window.close(),
            WindowEvent::Destroy => println!("Window destroyed"),
            WindowEvent::Visible(is_visible) => {
                if is_visible {
                    println!("Window is visible");
                } else {
                    println!("Window is hidden");
                }
            }
            WindowEvent::BeginResize => {
                println!("Window resize started on");
            }
            WindowEvent::Resize(size, scale) => {
                println!("Window resized to {} at {:?}", size, scale);
            }
            WindowEvent::EndResize => {
                println!("Window resize ended");
            }
            WindowEvent::Repaint(timings) => {
                println!("Window repaint requested for {:?}", timings.next_frame);
            }
            WindowEvent::PointerMove(_, _) => todo!(),
            WindowEvent::Scroll(_, _) => todo!(),
        }
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig {
        power_preference: PowerPreference::HighPerformance,
    });

    app.spawn_window(WindowSpec::default(), AppWindow::new);
    app.spawn_window(WindowSpec::default(), AppWindow::new);

    app.run();
}
