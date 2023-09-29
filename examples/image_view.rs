use plinth::{
    new_window, scene::Scene, Application, GraphicsConfig, Image, Panel, PowerPreference, Window,
    WindowEvent, WindowEventHandler, WindowSpec,
};

pub struct DemoWindow {
    window: Window,
}

impl DemoWindow {
    fn new(mut window: Window) -> Self {
        let background = Panel::new();
        let image = Image::from_path("path/to/image.png").unwrap();

        let mut scene = Scene::new();
        let (root, _) = scene.set_root(background);
        scene.add_child(root, image);

        window.set_scene(scene).unwrap();

        Self { window }
    }
}

impl WindowEventHandler for DemoWindow {
    fn event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Visible(is_visible) => {
                if is_visible {
                    self.window.begin_animation(None).unwrap();
                } else {
                    self.window.end_animation().unwrap();
                }
            }
            WindowEvent::Repaint(timings) => {
                // todo
            }
            WindowEvent::Scroll(axis, amount) => {
                // get cursor position
                // get image size

                // if image is same size as window, do nothing

                // calculate relative cursor position in image
                // let shrunk = calculate shrunk image size
                // move shrunk so that the cursor is in the same relative position

                // get image size
                // if image size is animating, stop stop animation at current location, and animate to shrunk
            }
            _ => {}
        }
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig {
        power_preference: PowerPreference::HighPerformance,
    });

    let _window = new_window(WindowSpec::default(), DemoWindow::new);

    app.run();
}
