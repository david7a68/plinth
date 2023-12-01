use std::time::Instant;

use clap::{command, Parser, ValueEnum};
use plinth::{
    animation::AnimationFrequency,
    application::{Application, GraphicsConfig},
    graphics::{Canvas, Color, Srgb},
    math::{Rect, Size, Translate},
    window::{Window, WindowEventHandler, WindowSpec},
};

struct DemoRect {
    rect: Rect<Canvas>,
    color: Color<Srgb>,
    velocity: Translate<Canvas, Canvas>,
}

struct DemoWindow {
    window: Window,
    rects: Vec<DemoRect>,
    throttle_animation: bool,
    last_present_time: Instant,
}

impl DemoWindow {
    fn new(mut window: Window, throttle_animation: bool) -> Self {
        let center = window.canvas().rect().center();

        let mut rects = Vec::new();
        for _ in 0..100 {
            let angle: f64 = rand::random::<f64>() * std::f64::consts::TAU;
            let (x, y) = angle.sin_cos();

            rects.push(DemoRect {
                rect: Rect::from_center(center, Size::new(100.0, 100.0)),
                color: Color::BLACK,
                velocity: Translate::new(x, y) * 2.0,
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
    fn on_close_request(&mut self) {
        self.window.close();
    }

    fn on_visible(&mut self, is_visible: bool) {
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
                Some(self.window.default_animation_frequency())

                // alternatively
                // Some(self.window.max_animation_frequency())
            };

            self.window.begin_animation(freq);
        } else {
            self.window.end_animation();
        }
    }

    fn on_repaint(&mut self, timings: plinth::animation::PresentTiming) {
        let delta = timings.next_frame - self.last_present_time;

        let canvas = self.window.canvas_mut();
        let canvas_rect = canvas.rect();

        for rect in &mut self.rects {
            rect.rect += rect.velocity * delta;

            if let Some(intersection) = canvas_rect.intersection(&rect.rect) {
                // reverse rect direction
                rect.velocity = -rect.velocity;

                // snap it into the self.window
                rect.rect.x -= intersection.width;
                rect.rect.y -= intersection.height;
            }
        }

        // Request a drawing context for the self.window, constrained to the
        // dirty rectangle provided.
        canvas.clear(Color::BLACK);

        for rect in &self.rects {
            canvas.draw_rect(rect.rect, rect.color.into());
        }

        // canvas repaint
        self.last_present_time = timings.next_frame;
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

    let mut app = Application::new(&GraphicsConfig::default());

    match args.count {
        Count::One => run_one(&mut app, throttle),
        Count::Many => run_many(&mut app, throttle),
    }
}

fn make_demo_window(throttle: bool) -> impl Fn(Window) -> DemoWindow {
    move |window| DemoWindow::new(window, throttle)
}

fn run_one(app: &mut Application, throttle: bool) {
    app.spawn_window(
        WindowSpec {
            resizable: false,
            ..Default::default()
        },
        make_demo_window(throttle),
    );
    app.run();
}

fn run_many(app: &mut Application, throttle: bool) {
    let spec = WindowSpec::default();
    app.spawn_window(spec.clone(), make_demo_window(throttle));
    app.spawn_window(spec.clone(), make_demo_window(throttle));
    app.spawn_window(spec, make_demo_window(throttle));
    app.run();
}
