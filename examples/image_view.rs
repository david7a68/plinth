use plinth::{
    application::{Application, GraphicsConfig, PowerPreference},
    math::Rect,
    visuals::{Canvas, FromVisual, Image, VisualTree},
    window::{Window, WindowEvent, WindowEventHandler, WindowSpec},
};

const SCROLL_SCALE: f64 = 1.1;

pub struct DemoWindow {
    window: Window,
}

impl DemoWindow {
    fn new(mut window: Window) -> Self {
        let mut scene = VisualTree::new();
        let (root, _) = scene.set_root(Canvas::new(window.size() * window.scale()));

        scene.add_child(root, Image::from_path("path/to/image.png").unwrap());

        window.set_scene(scene);

        Self { window }
    }
}

impl WindowEventHandler for DemoWindow {
    fn event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Visible(is_visible) => {
                if is_visible {
                    self.window.begin_animation(None);
                } else {
                    self.window.end_animation();
                }
            }
            WindowEvent::Resize(size, scale) => {
                self.window
                    .scene_mut()
                    .root_mut::<Canvas>()
                    .unwrap()
                    .set_rect(size * scale);

                // leave image size as-is. Could also scale it proportionally, but meh.
            }
            WindowEvent::Repaint(_timings) => {
                // todo
            }
            WindowEvent::Scroll(_axis, amount) => {
                let pointer = self.window.pointer_location() * self.window.scale();
                let scene = self.window.scene_mut();
                let target = scene.hit_test_mut(pointer);

                if let Some((_, target)) = target {
                    let Some(image) = Image::from_mut(target) else {
                        return;
                    };

                    let image_rect = image.rect();
                    let pointer_offset = pointer - image_rect.top_left();

                    let new_size = image_rect.size() * amount as f64 * SCROLL_SCALE;
                    let new_offset = pointer_offset * amount as f64 * SCROLL_SCALE - pointer_offset;
                    let new_origin = image_rect.top_left() - new_offset;

                    image.set_rect(Rect::from_origin(new_origin, new_size));
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig {
        power_preference: PowerPreference::HighPerformance,
    });

    app.spawn_window(&WindowSpec::default(), DemoWindow::new);

    app.run();
}
