use plinth::{
    application::{Application, GraphicsConfig},
    graphics::{Canvas, FrameInfo, FramesPerSecond},
    input::Axis,
    window::{Window, WindowEventHandler, WindowSpec},
};

const SCROLL_SCALE: f64 = 1.1;

pub struct DemoWindow {
    window: Window,
}

impl DemoWindow {
    fn new(window: Window) -> Self {
        Self { window }
    }
}

impl WindowEventHandler for DemoWindow {
    fn on_close_request(&mut self) {
        self.window.close();
    }

    fn on_visible(&mut self, visible: bool) {
        if visible {
            self.window
                .set_animation_frequency(self.window.refresh_rate().optimal_fps);
        } else {
            self.window.set_animation_frequency(FramesPerSecond::ZERO);
        }
    }

    fn on_resize(
        &mut self,
        size: plinth::math::Size<Window>,
        scale: plinth::math::Scale<Window, Window>,
    ) {
    }

    fn on_repaint(&mut self, canvas: &mut Canvas<Window>, _timing: &FrameInfo) {
        // todo
    }

    fn on_scroll(&mut self, _axis: Axis, delta: f32) {
        // todo: commented out because scenes have been removed

        // let pointer = self.window.pointer_location() * self.window.scale();
        // let scene = self.window.scene_mut();
        // let target = scene.hit_test_mut(pointer);

        // if let Some((_, target)) = target {
        //     let Some(image) = Image::from_mut(target) else {
        //         return;
        //     };

        //     let image_rect = image.rect();
        //     let pointer_offset = pointer - image_rect.top_left();

        //     let new_size = image_rect.size() * delta as f64 * SCROLL_SCALE;
        //     let new_offset = pointer_offset * delta as f64 * SCROLL_SCALE - pointer_offset;
        //     let new_origin = image_rect.top_left() - new_offset;

        //     image.set_rect(Rect::from_origin(new_origin, new_size));
        // }
    }
}

fn main() {
    let mut app = Application::new(&GraphicsConfig::default());
    app.spawn_window(WindowSpec::default(), DemoWindow::new);
    app.run();
}
