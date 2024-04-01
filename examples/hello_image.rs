use plinth::{
    geometry::{Extent, Texel},
    graphics::{
        Canvas, Color, Format, FrameInfo, GraphicsConfig, ImageInfo, Layout, RasterBuf, RoundRect,
    },
    hashed_str,
    resource::StaticResource,
    system::{Window, WindowAttributes},
    AppContext, Application, Config, EventHandler,
};

#[rustfmt::skip]
const IMAGE: RasterBuf<'static> = RasterBuf::new(
    ImageInfo {
        extent: Extent {width: Texel(3), height: Texel(1)},
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
    };

    Application::new(&config).unwrap().run(App {}).unwrap();
}

pub struct AppWindow {}

pub struct App {}

impl EventHandler<AppWindow> for App {
    fn start(&mut self, app: &mut AppContext<AppWindow>) {
        app.create_window(WindowAttributes::default(), |_| AppWindow {})
            .unwrap();
    }

    fn stop(&mut self) {
        // no-op
    }

    fn wake_requested(
        &mut self,
        _app: &mut AppContext<AppWindow>,
        _window: &mut Window<AppWindow>,
    ) {
        // no-op
    }

    fn destroyed(&mut self, _app: &mut AppContext<AppWindow>, _window_data: AppWindow) {
        // no-op
    }

    fn repaint(
        &mut self,
        app: &mut AppContext<AppWindow>,
        _window: &mut Window<AppWindow>,
        canvas: &mut Canvas,
        _frame: &FrameInfo,
    ) {
        let image = app.load_image(hashed_str!("image")).unwrap();

        canvas.clear(Color::WHITE);
        canvas.draw_rect(&RoundRect::new((50.0, 100.0, 40.0, 70.0)).with_image(image));
        canvas.draw_rect(&RoundRect::new((100.0, 100.0, 40.0, 70.0)).with_color(Color::RED));
    }
}
