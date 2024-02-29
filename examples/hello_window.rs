use plinth::{
    graphics::{Canvas, FrameInfo, GraphicsConfig},
    AppContext, Application, ButtonState, DpiScale, EventHandler, KeyCode, ModifierKeys,
    MouseButton, ScrollAxis, Window, WindowAttributes, WindowPoint, WindowSize,
};

pub struct App {}

#[allow(unused_variables)]
impl EventHandler<()> for App {
    fn start(&mut self, app: &AppContext<()>) {
        app.create_window(WindowAttributes::default(), |_| {
            println!("Window created");
        })
        .unwrap();

        app.create_window(WindowAttributes::default(), |_| {
            println!("Window created");
        })
        .unwrap();
    }

    fn suspend(&mut self, app: &AppContext<()>) {
        println!("Event loop suspended");
    }

    fn resume(&mut self, app: &AppContext<()>) {
        println!("Event loop resumed");
    }

    fn stop(&mut self) {
        println!("Event loop stopped");
    }

    fn low_memory(&mut self, app: &AppContext<()>) {
        println!("Low memory event");
    }

    fn power_source_changed(
        &mut self,
        app: &AppContext<()>,
        power_source: plinth::system::power::PowerSource,
    ) {
        println!("Power source changed: {:?}", power_source);
    }

    fn monitor_state_changed(
        &mut self,
        app: &AppContext<()>,
        monitor: plinth::system::power::MonitorState,
    ) {
        println!("Monitor state changed: {:?}", monitor);
    }

    fn power_preference_changed(
        &mut self,
        app: &AppContext<()>,
        power_preference: plinth::system::power::PowerPreference,
    ) {
        println!("Power preference changed: {:?}", power_preference);
    }

    fn activated(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window activated");
    }

    fn deactivated(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window deactivated");
    }

    fn drag_resize_started(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window drag resize started");
    }

    fn drag_resize_ended(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window drag resize ended");
    }

    fn resized(&mut self, app: &AppContext<()>, window: &mut Window<()>, size: WindowSize) {
        println!("Window resized: {:?}", size);
    }

    fn dpi_changed(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        dpi: DpiScale,
        size: WindowSize,
    ) {
        println!("Window DPI changed: {:?}", dpi);
        println!("Window size: {:?}", size);
    }

    fn close_requested(&mut self, _app: &AppContext<()>, mut window: &mut Window<()>) {
        println!("Window close request");
        window.destroy();
    }

    fn shown(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window shown");
    }

    fn hidden(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window hidden");
    }

    fn maximized(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window maximized");
    }

    fn minimized(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window minimized");
    }

    fn restored(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window restored");
    }

    fn moved(&mut self, app: &AppContext<()>, window: &mut Window<()>, position: WindowPoint) {
        println!("Window moved: {:?}", position);
    }

    fn wake_requested(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Window wake requested");
    }

    fn repaint(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        canvas: &mut Canvas,
        frame_info: &FrameInfo,
    ) {
        println!("Window needs repaint");
    }

    fn destroyed(&mut self, _app: &AppContext<()>, _window_data: ()) {
        println!("Window destroyed");
    }

    fn key(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    ) {
        println!("Key input: {:?} {:?} {:?}", code, state, modifiers);
    }

    fn mouse_button(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        button: MouseButton,
        state: ButtonState,
        position: WindowPoint,
        modifiers: ModifierKeys,
    ) {
        println!(
            "Mouse button input: {:?} {:?} {:?} {:?}",
            button, state, position, modifiers
        );
    }

    fn pointer_moved(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        position: WindowPoint,
    ) {
        println!("Mouse moved: {:?}", position);
    }

    fn pointer_entered(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        position: WindowPoint,
    ) {
        println!("Mouse entered: {:?}", position);
    }

    fn pointer_left(&mut self, app: &AppContext<()>, window: &mut Window<()>) {
        println!("Mouse left");
    }

    fn mouse_scrolled(
        &mut self,
        app: &AppContext<()>,
        window: &mut Window<()>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
        println!("Mouse scrolled: {:?} {:?} {:?}", delta, axis, modifiers);
    }
}

pub fn main() {
    let graphics_config = GraphicsConfig::default();
    let mut event_loop = Application::new(&graphics_config).unwrap();
    event_loop.run(App {}).unwrap();
}
