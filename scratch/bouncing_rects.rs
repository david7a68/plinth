mod plinth {
    struct PresentTiming {
        next_frame: Instant,
        last_frame: Instant,
    }

    struct AnimationFrequency {
        /// The minimum rate at which the window should be animated.
        min_fps: Option<f32>,
        /// The maximum rate at which the window should be animated.
        max_fps: Option<f32>,
        /// The optimal rate at which the window should be animated.
        optimal_fps: f32,
    }

    #[non_exhaustive]
    pub enum WindowEvent {
        Visible(bool),
        Repaint(Gfx, Rect, PresentTiming),
    }

    pub trait Window {
        fn event(&mut self, event: WindowEvent);
    }

    pub fn new_window(spec: WindowSpec, handler: impl Window + 'static) -> WindowHandle {
        todo!()
    }

    pub fn run_all() {
        todo!()
    }

    pub fn run(spec: WindowSpec, handler: impl Window + 'static) {
        todo!()
    }
}

use plinth::{
    new_window, run, run_all, AnimationFrequency, PresentTiming, Window, WindowEvent, WindowHandle,
    WindowSpec,
};

struct DemoRect {
    rect: Rect,
    color: Color,
    velocity: Vector,
}

struct DemoWindow {
    window: WindowHandle,
    rects: [DemoRect; 1000],
    throttle_animation: bool,
    last_present_time: Instant,
}

impl DemoWindow {
    fn new(window: WindowHandle) -> Self {
        let rects = todo!("random rect locations and velocities");

        Self {
            window,
            rects,
            last_present_time: Instant::now(),
        }
    }
}

impl Window for DemoWindow {
    fn event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Visible(is_visible) => {
                if is_visible {
                    let freq = if self.throttle_animation {
                        // throttle to <= 30fps (might be e.g. 28.8 fps on a 144
                        // hz display at 1/5 refresh rate)
                        Some(AnimationFrequency {
                            min_fps: None,
                            max_fps: Some(30.0),
                            optimal_fps: 30.0,
                        })
                    } else {
                        // No throttling, default to display refresh rate. This
                        // is a polite fiction, since the display refresh rate
                        // may change at any time.
                        Some(window.default_animation_frequency())

                        // alternatively
                        // Some(window.max_animation_frequency())
                    };

                    window.begin_animation(freq);
                } else {
                    window.end_animation();
                }
            }
            WindowEvent::Repaint(gfx, dirty_rect, timings) => {
                let delta = timings.next_frame - self.last_present_time;

                for rects in &self.rects {
                    rects.rect.translate(rects.velocity * delta);

                    if !window.rect().contains(rects.rect) {
                        // reverse rect direction
                        rects.rect.velocity = -rects.rect.velocity;

                        // snap it into the window
                        window.rect().snap(&mut rects.rect);
                    }
                }

                // Request a drawing context for the window, constrained to the
                // dirty rectangle provided.
                let mut frame = gfx.new_frame(&window, Some(rect));

                frame.clear(Color::BLACK);

                for rect in &self.rects {
                    // automatically clips to the dirty rect
                    frame.fill(rect.rect, rect.color);
                }

                // Present the frame to the window at the appropriate time.
                gfx.present(frame, timings.next_frame);

                self.last_present_time = timings.next_frame;
            }
            _ => {}
        }
    }
}

fn main() {
    // run_one();
    run_many();
}

fn run_one() {
    // convenience function: create window and run until all windows are closed
    // (b.c. the window could create more)
    plinth::run(WindowSpec::default(), &DemoWindow::new);
}

fn run_many() {
    let a = plinth::new_window(WindowSpec::default(), &DemoWindow::new);
    let b = plinth::new_window(WindowSpec::default(), &DemoWindow::new);
    let c = plinth::new_window(WindowSpec::default(), &DemoWindow::new);
    plinth::run_all();

    // run 3 windows to completion, then run one more

    let d = plinth::new_window(WindowSpec::default(), &DemoWindow::new);
    plinth::run_all();
}
