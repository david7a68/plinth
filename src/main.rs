mod graphics;
mod window;

use std::{mem::MaybeUninit, sync::Arc};

use crate::graphics::*;
use crate::window::*;
use crossbeam::queue::ArrayQueue;
use euclid::Size2D;

use parking_lot::Mutex;
use tracing::info;
use windows::Win32::Graphics::DirectComposition::DCompositionWaitForCompositorClock;

#[cfg(feature = "profile")]
use tracing_tracy::client::{plot, span_location};

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
        info!("begin_resize");
        self.is_resizing = true;
    }

    pub fn end_resize(&mut self) {
        info!("end_resize");
        self.is_resizing = false;
    }

    #[tracing::instrument(skip(self, device))]
    pub fn paint(&mut self, device: &mut graphics::Device) {
        let size = self.handle.content_size();

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

fn start_ui_thread(graphics_config: GraphicsConfig) -> UiThreadProxy {
    let window_id_pool = Arc::new(Mutex::new(WindowIdPool::new()));
    let event_queue: Arc<ArrayQueue<(WindowId, Event)>> = Arc::new(ArrayQueue::new(128));

    let event_queue_ = event_queue.clone();
    let window_id_pool_ = window_id_pool.clone();

    std::thread::spawn(move || {
        #[cfg(feature = "profile")]
        let tracy = {
            let tracy = tracing_tracy::client::Client::start();
            tracy.set_thread_name("UI thread");
            tracy
        };

        let event_queue = event_queue_;
        let window_id_pool = window_id_pool_;
        let mut window_pool = Vec::<MaybeUninit<AppWindow>>::new();

        let mut graphics = graphics::Device::new(&graphics_config);

        #[tracing::instrument(skip_all)]
        fn handle_events(
            window_pool: &mut Vec<MaybeUninit<AppWindow>>,
            window_id_pool: &Arc<Mutex<WindowIdPool>>,
            graphics: &mut graphics::Device,
            event_queue: &ArrayQueue<(WindowId, Event)>,
        ) {
            #[cfg(feature = "profile")]
            plot!("ui thread events", event_queue.len() as f64);

            while let Some((window_id, event)) = event_queue.pop() {
                let window_id_ = window_id.0 as usize;

                match event {
                    Event::Create(handle) => {
                        let window = AppWindow::new(&graphics, handle);

                        while window_pool.len() <= window_id_ {
                            window_pool.push(MaybeUninit::uninit());
                        }

                        window_pool[window_id_] = MaybeUninit::new(window);
                    }
                    Event::Destroy => {
                        let mut window = unsafe { window_pool[window_id_].assume_init_read() };
                        window.destroy(graphics);
                        window_id_pool.lock().release_id(window_id);
                    }
                    Event::Close => unsafe { window_pool[window_id_].assume_init_mut().close() },
                    Event::ResizeBegin => unsafe {
                        window_pool[window_id_].assume_init_mut().begin_resize()
                    },
                    Event::ResizeEnd => unsafe {
                        window_pool[window_id_].assume_init_mut().end_resize()
                    },
                    Event::Paint(_) => {
                        //     window_pool[0]
                        //         .assume_init_mut()
                        //         .paint(graphics, size)
                    }
                }

                #[cfg(feature = "profile")]
                plot!("ui thread events", event_queue.len() as f64);
            }
        }

        #[tracing::instrument(skip_all)]
        fn draw_windows(
            window_pool: &mut Vec<MaybeUninit<AppWindow>>,
            graphics: &mut graphics::Device,
        ) {
            unsafe { window_pool[0].assume_init_mut().paint(graphics) };
        }

        loop {
            handle_events(
                &mut window_pool,
                &window_id_pool,
                &mut graphics,
                &event_queue,
            );

            draw_windows(&mut window_pool, &mut graphics);

            unsafe { DCompositionWaitForCompositorClock(None, u32::MAX) };

            #[cfg(feature = "profile")]
            tracy.frame_mark();
        }
    });

    UiThreadProxy {
        window_pool: window_id_pool,
        event_queue,
    }
}

struct WindowIdPool {
    free_indices: Vec<u32>,
}

impl WindowIdPool {
    pub fn new() -> Self {
        Self {
            free_indices: Vec::new(),
        }
    }

    pub fn reserve_id(&mut self) -> WindowId {
        if let Some(index) = self.free_indices.pop() {
            WindowId(index as u64)
        } else {
            let index = self.free_indices.len();
            WindowId(index as u64)
        }
    }

    pub fn release_id(&mut self, id: WindowId) {
        let index = id.0 as usize;
        self.free_indices.push(index as u32);
    }
}

struct UiThreadProxy {
    window_pool: Arc<Mutex<WindowIdPool>>,
    event_queue: Arc<ArrayQueue<(WindowId, Event)>>,
}

impl EventSink for UiThreadProxy {
    fn new_window(&mut self) -> WindowId {
        self.window_pool.lock().reserve_id()
    }

    fn send(&mut self, window: WindowId, event: Event) {
        if let Err(_) = self.event_queue.push((window, event)) {
            panic!("UI thread event queue full");
        }

        #[cfg(feature = "profile")]
        plot!("ui thread events", self.event_queue.len() as f64);
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

    let graphics_config = GraphicsConfig { debug_mode: false };
    let ui_proxy = start_ui_thread(graphics_config);

    let event_loop = EventLoop::new(ui_proxy);

    let _window1 = WindowSpec {
        title: "Oh look, windows!",
        size: Size2D::new(800, 600),
    }
    .build();

    event_loop.run()
}
