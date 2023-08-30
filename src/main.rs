mod graphics;
mod window;

use crate::graphics::*;
use crate::window::*;
use euclid::Size2D;

/// Determines how quickly swapchain buffers grow when the window is resized.
/// This helps to amortize the cost of calling `ResizeBuffers` on every frame.
/// Once resized, the swapchain is set to the size of the window to conserve
/// memory.
const SWAPCHAIN_GROWTH_FACTOR: f32 = 1.2;

struct AppWindow {
    handle: WindowHandle,
    swapchain: Swapchain,

    is_resizing: bool,
    content_size: Size2D<u16, ScreenSpace>,
}

impl AppWindow {
    pub fn new(device: &graphics::Device, handle: WindowHandle) -> Self {
        let swapchain = device.create_swapchain(handle.hwnd());
        let content_size = handle.content_size();

        handle.show();

        Self {
            handle,
            swapchain,
            is_resizing: false,
            content_size,
        }
    }

    pub fn close(&mut self) {
        self.handle.destroy();
    }

    pub fn destroy(&mut self, device: &mut graphics::Device) {
        device.flush();
    }

    pub fn begin_resize(&mut self) {
        self.is_resizing = true;
    }

    pub fn end_resize(&mut self) {
        self.is_resizing = false;
    }

    pub fn paint(&mut self, device: &mut graphics::Device, size: Size2D<u16, ScreenSpace>) {
        if self.content_size != size {
            if self.is_resizing {
                self.swapchain.resize(ResizeOp::Flex {
                    size: size,
                    flex: SWAPCHAIN_GROWTH_FACTOR,
                });
            } else {
                self.swapchain.resize(ResizeOp::Auto);
            }

            self.content_size = size;
        }

        let (image, _) = self.swapchain.get_back_buffer();
        let canvas = device.create_canvas(image);
        device.draw_canvas(canvas);
        self.swapchain.present();
    }
}

struct DummySink {
    device: graphics::Device,
    window: Option<AppWindow>,
}

impl DummySink {
    fn new(device: graphics::Device) -> Self {
        Self {
            device,
            window: None,
        }
    }

    fn new_id(&mut self) -> WindowId {
        WindowId(0)
    }
}

impl EventSink for DummySink {
    fn send(&mut self, window: WindowId, event: Event) {
        match event {
            Event::Create(handle) => {
                self.window = Some(AppWindow::new(&self.device, handle));
            }
            Event::Close => {
                self.window.as_mut().unwrap().close();
            }
            Event::Destroy => {
                self.window.as_mut().unwrap().destroy(&mut self.device);
                let _ = self.window.take();
            }
            Event::ResizeBegin => {
                self.window.as_mut().unwrap().begin_resize();
            }
            Event::ResizeEnd => {
                self.window.as_mut().unwrap().end_resize();
            }
            Event::Paint(size) => {
                self.window.as_mut().unwrap().paint(&mut self.device, size);
            }
        }
    }
}

impl Drop for DummySink {
    fn drop(&mut self) {
        self.device.flush();
    }
}

fn main() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let graphics_config = GraphicsConfig { debug_mode: true };

    let graphics = graphics::Device::new(&graphics_config);

    let mut sink = DummySink::new(graphics);
    let id = sink.new_id();

    EventLoop::run(sink, || {
        let _window1 = WindowSpec {
            title: "Oh look, windows!",
            size: Size2D::new(800, 600),
        }
        .build(id);
    });
}
