use plinth::{
    math::Rect,
    visuals::{FromVisual, Image, Panel, VisualTree},
    Application, GraphicsConfig, PowerPreference, Window, WindowEvent, WindowEventHandler,
    WindowSpec,
};

const SCROLL_SCALE_FACTOR: f64 = 1.1;

pub struct DemoWindow {
    window: Window,
}

impl DemoWindow {
    fn new(mut window: Window) -> Self {
        let background = Panel::new();
        let image = Image::from_path("path/to/image.png").unwrap();

        let mut scene = VisualTree::new();
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
            WindowEvent::Repaint(_timings) => {
                // todo
            }
            WindowEvent::Scroll(_axis, amount) => {
                let pointer = self.window.pointer_location().unwrap();
                let scene = self.window.scene_mut().unwrap();
                let target = scene.hit_test_mut(pointer);

                if let Some((target_id, target)) = target {
                    let Some(_image) = Image::from_mut(target) else {
                        return;
                    };

                    let image_rect = scene.view_rect(target_id).unwrap();
                    let pointer_offset = pointer - image_rect.top_left();

                    let new_size = image_rect.size() * (amount as f64 * SCROLL_SCALE_FACTOR).into();
                    let new_offset = pointer_offset * (amount as f64 * SCROLL_SCALE_FACTOR).into()
                        - pointer_offset;
                    let new_origin = image_rect.top_left() - new_offset;

                    scene.set_view_rect(target_id, Rect::from_origin(new_origin, new_size));
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

    app.create_window(&WindowSpec::default(), DemoWindow::new)
        .unwrap();

    app.run();
}
