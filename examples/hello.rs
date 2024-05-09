use plinth::{
    geometry::{Extent, Point, Rect},
    graphics::{
        Canvas, Color, FontOptions, FontWeight, Format, FrameInfo, GraphicsConfig, ImageExtent,
        ImageInfo, Layout, Pt, RasterBuf, RoundRect, TextBox, TextLayout, TextWrapMode,
    },
    hashed_str,
    resource::StaticResource,
    system::{Window, WindowAttributes},
    AppContext, Application, Config, EventHandler,
};

#[rustfmt::skip]
const IMAGE: RasterBuf<'static> = RasterBuf::new(
    ImageInfo {
        extent: ImageExtent {width: 3, height: 1},
        format: Format::Linear,
        layout: Layout::Rgba8,
    },
    &[
        255, 0, 0, 255,
        0, 255, 0, 255,
        0, 0, 255, 255
    ],
);

const RESOURCES: &[StaticResource] = &[StaticResource::Raster(hashed_str!("image"), IMAGE)];

fn main() {
    let config = Config {
        graphics: GraphicsConfig {
            debug_mode: true,
            ..Default::default()
        },
        resources: RESOURCES,
        ..Default::default()
    };

    // let text_engine = TextEngine::new();

    // let glyph_cache = text_engine.create_glyph_cache();

    Application::new(&config)
        .unwrap()
        .run(App {
        //text_engine,
    })
        .unwrap();
}

pub struct AppWindow {
    // hello_world: TextLayout,
}

pub struct App {
    // text_engine: TextEngine,
}

impl EventHandler<AppWindow> for App {
    fn start(&mut self, app: &mut AppContext<AppWindow>) {
        // let hello_world = app.layout_text(
        //     "Hello, World!",
        //     FontOptions {
        //         name: hashed_str!("Arial"),
        //         size: Pt(40),
        //         weight: FontWeight::Bold,
        //         ..Default::default()
        //     },
        //     TextBox {
        //         wrap: TextWrapMode::Word,
        //         extent: Extent::new(1000.0, 100.0),
        //         line_spacing: 0.8,
        //     },
        // );

        app.create_window(WindowAttributes::default(), |_| AppWindow {
            // hello_world
        })
        .unwrap();
    }

    fn stop(&mut self, _app: &mut AppContext<AppWindow>) {
        // no-op
    }

    fn window_wake_requested(
        &mut self,
        _app: &mut AppContext<AppWindow>,
        _window: &mut Window<AppWindow>,
    ) {
        // no-op
    }

    fn window_destroyed(&mut self, _app: &mut AppContext<AppWindow>, _window_data: AppWindow) {
        // no-op
    }

    fn window_frame(
        &mut self,
        app: &mut AppContext<AppWindow>,
        window: &mut Window<AppWindow>,
        canvas: &mut Canvas,
        _frame: &FrameInfo,
    ) {
        let image = app.load_image(hashed_str!("image")).unwrap();

        canvas.clear(Color::WHITE);
        canvas.draw_rect(
            RoundRect::new(Rect::new(Point::new(50.0, 100.0), Extent::new(40.0, 70.0)))
                .with_image(image),
        );
        canvas.draw_rect(
            RoundRect::new(Rect::new(Point::new(100.0, 100.0), Extent::new(40.0, 70.0)))
                .with_color(Color::RED),
        );

        // canvas.draw_text_layout(&window.hello_world, Point::new(50.0, 150.0));

        canvas.draw_text(
            "Hello, World!",
            &FontOptions {
                name: hashed_str!("Arial"),
                size: Pt(40),
                weight: FontWeight::Bold,
                ..Default::default()
            },
            &TextBox {
                wrap: TextWrapMode::Word,
                extent: Extent::new(1000.0, 100.0),
                line_spacing: 0.8,
            },
            Point::new(50.0, 150.0),
        );
    }
}
