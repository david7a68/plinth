use plinth::{
    graphics::{Canvas, Color, FrameInfo, GraphicsConfig},
    Application, Input, Window, WindowEvent, WindowEventHandler, WindowSpec,
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
    fn on_event(&mut self, event: plinth::WindowEvent) {
        match event {
            WindowEvent::CloseRequest => {
                println!("Close request");
                self.window.close();
            }
            WindowEvent::Visible(is_visible) => {
                println!("Visible: {}", is_visible);
            }
            WindowEvent::BeginResize => {
                println!("Begin resize");
            }
            WindowEvent::Resize(size) => {
                println!("Resize: {:?}", size);
            }
            WindowEvent::EndResize => {
                println!("End resize");
            }
        }
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::MouseButton(button, state, position) => {
                println!("Mouse button {:?} {:?} at {:?}", button, state, position);
            }
            Input::PointerMove(position) => {
                println!("Pointer move to {:?}", position);
            }
            Input::PointerLeave => {
                println!("Pointer leave");
            }
            Input::Scroll(axis, delta) => {
                println!("Scroll {:?} by {}", axis, delta);
            }
        }
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, timing: &FrameInfo) {
        println!("Repaint at {:?}", timing);
        canvas.clear(Color::GREEN);
    }
}

impl Drop for AppWindow {
    fn drop(&mut self) {
        println!("Destroy");
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig::default());

    app.spawn_window(WindowSpec::default(), AppWindow::new)
        .unwrap();

    app.run();
}
