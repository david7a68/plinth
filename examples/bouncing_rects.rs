use std::time::Instant;

use clap::{command, Parser, ValueEnum};

use plinth::{
    new_window, AnimationFrequency, Application, Canvas, Color, GraphicsConfig, Pixel,
    PixelsPerSecond, PowerPreference, Rect2D, SceneNode, Size2D, Srgb, Translate2D, Window,
    WindowEvent, WindowEventHandler, WindowSpec,
};

struct DemoRect {
    rect: Rect2D<Pixel>,
    color: Color<Srgb>,
    velocity: Translate2D<PixelsPerSecond>,
}

struct DemoWindow {
    window: Window,
    rects: Vec<DemoRect>,
    throttle_animation: bool,
    last_present_time: Instant,
}

impl DemoWindow {
    fn new(mut window: Window, throttle_animation: bool) -> Self {
        window.set_scene(SceneNode::Canvas(Canvas::new())).unwrap();

        let window_center = Rect2D::from(window.size().unwrap().logical).center();

        let mut rects = Vec::new();
        for _ in 0..100 {
            let angle: f32 = rand::random::<f32>() * std::f32::consts::TAU;
            let (x, y) = angle.sin_cos();

            rects.push(DemoRect {
                rect: Rect2D::centered_on(window_center.cast::<Pixel>(), Size2D::new(100.0, 100.0)),
                color: Color::BLACK,
                velocity: Translate2D::new(x, y) * 2.0,
            });
        }

        Self {
            window,
            rects,
            throttle_animation,
            last_present_time: Instant::now(),
        }
    }
}

impl WindowEventHandler for DemoWindow {
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
                        Some(self.window.default_animation_frequency().unwrap())

                        // alternatively
                        // Some(self.window.max_animation_frequency())
                    };

                    self.window.begin_animation(freq).unwrap();
                } else {
                    self.window.end_animation().unwrap();
                }
            }
            WindowEvent::Repaint(timings) => {
                let delta = (timings.next_frame - self.last_present_time).as_secs_f32();
                let window_rect = Rect2D::from(self.window.size().unwrap().logical);

                for rect in &mut self.rects {
                    rect.rect.translate(rect.velocity * delta);

                    if window_rect.intersection(&rect.rect).is_none() {
                        // reverse rect direction
                        rect.velocity = -rect.velocity;

                        // snap it into the self.window
                        rect.rect.move_into(&window_rect);
                    }
                }

                let scene = self.window.scene_mut().unwrap();
                let SceneNode::Canvas(canvas) = scene else {
                    panic!()
                };

                // Request a drawing context for the self.window, constrained to the
                // dirty rectangle provided.
                canvas.clear(Color::BLACK);

                for rect in &self.rects {
                    canvas.fill(rect.rect, rect.color);
                }

                // canvas repaint
                self.last_present_time = timings.next_frame;
            }
            _ => {}
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
enum Count {
    One,
    Many,
}

#[derive(Parser, Debug)]
#[command(author, version)]
struct Cli {
    #[arg(short, long, default_value = "true")]
    throttle_animation: bool,

    #[arg(short, long, default_value = "Count::One")]
    count: Count,
}

fn main() {
    let args = Cli::parse();
    let throttle = args.throttle_animation;

    let mut app = Application::new(&GraphicsConfig {
        power_preference: PowerPreference::HighPerformance,
    });

    match args.count {
        Count::One => run_one(&mut app, throttle),
        Count::Many => run_many(&mut app, throttle),
    }
}

fn make_demo_window(throttle: bool) -> impl Fn(Window) -> DemoWindow {
    move |window| DemoWindow::new(window, throttle)
}

fn run_one(app: &mut Application, throttle: bool) {
    let _window = new_window(WindowSpec::default(), make_demo_window(throttle));
    app.run();
}

fn run_many(app: &mut Application, throttle: bool) {
    let _a = new_window(WindowSpec::default(), make_demo_window(throttle));
    let _b = new_window(WindowSpec::default(), make_demo_window(throttle));
    let _c = new_window(WindowSpec::default(), make_demo_window(throttle));
    app.run();
}
