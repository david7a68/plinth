use plinth::{
    graphics::{Canvas, FrameInfo, GraphicsConfig},
    system::{
        InputEvent, MonitorState, PowerPreference, PowerSource, Window, WindowAttributes,
        WindowPoint,
    },
    AppContext, Application, Config, EventHandler, PowerStateHandler, WindowFrameHandler,
};

pub fn main() {
    let config = Config {
        graphics: GraphicsConfig {
            debug_mode: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut event_loop = Application::new(&config).unwrap();
    event_loop.run(App {}).unwrap();
}

pub struct App {}

#[allow(unused_variables)]
impl EventHandler<()> for App {
    fn start(&mut self, app: &mut AppContext<()>) {
        app.create_window(WindowAttributes::default(), |_| {
            println!("Window created");
        })
        .unwrap();

        app.create_window(WindowAttributes::default(), |_| {
            println!("Window created");
        })
        .unwrap();
    }

    fn suspend(&mut self, app: &mut AppContext<()>) {
        println!("Event loop suspended");
    }

    fn resume(&mut self, app: &mut AppContext<()>) {
        println!("Event loop resumed");
    }

    fn stop(&mut self, app: &mut AppContext<()>) {
        println!("Event loop stopped");
    }

    fn low_memory(&mut self, app: &mut AppContext<()>) {
        println!("Low memory event");
    }

    fn window_close_requested(&mut self, _app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window close request");
        window.destroy();
    }

    fn window_wake_requested(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window wake requested");
    }

    fn window_frame(
        &mut self,
        app: &mut AppContext<()>,
        window: &mut Window<()>,
        canvas: &mut Canvas,
        frame_info: &FrameInfo,
    ) {
        println!("Window needs repaint");
    }

    fn window_destroyed(&mut self, _app: &mut AppContext<()>, _window_data: ()) {
        println!("Window destroyed");
    }

    fn window_input(
        &mut self,
        app: &mut AppContext<()>,
        window: &mut Window<()>,
        event: InputEvent,
    ) {
        println!("Window input event: {:?}", event);
    }

    fn power_state_handler(&mut self) -> Option<&mut dyn PowerStateHandler<()>> {
        Some(self)
    }

    fn window_frame_handler(&mut self) -> Option<&mut dyn WindowFrameHandler<()>> {
        Some(self)
    }
}

#[allow(unused_variables)]
impl PowerStateHandler<()> for App {
    fn power_source_changed(&mut self, app: &mut AppContext<()>, power_source: PowerSource) {
        println!("Power source changed: {:?}", power_source);
    }

    fn monitor_state_changed(&mut self, app: &mut AppContext<()>, monitor: MonitorState) {
        println!("Monitor state changed: {:?}", monitor);
    }

    fn power_preference_changed(
        &mut self,
        app: &mut AppContext<()>,
        power_preference: PowerPreference,
    ) {
        println!("Power preference changed: {:?}", power_preference);
    }
}

#[allow(unused_variables)]
impl WindowFrameHandler<()> for App {
    fn activated(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window activated");
    }

    fn deactivated(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window deactivated");
    }

    fn drag_resize_started(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window drag resize started");
    }

    fn drag_resize_ended(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window drag resize ended");
    }

    fn shown(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window shown");
    }

    fn hidden(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window hidden");
    }

    fn maximized(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window maximized");
    }

    fn minimized(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window minimized");
    }

    fn restored(&mut self, app: &mut AppContext<()>, window: &mut Window<()>) {
        println!("Window restored");
    }

    fn moved(&mut self, app: &mut AppContext<()>, window: &mut Window<()>, position: WindowPoint) {
        println!("Window moved: {:?}", position);
    }
}
