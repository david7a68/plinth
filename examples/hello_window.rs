use plinth::system::{
    dpi::{DpiScale, WindowPoint, WindowSize},
    input::{ButtonState, KeyCode, ModifierKeys, MouseButton, ScrollAxis},
    window::{Window, WindowAttributes},
    ActiveEventLoop, EventHandler, EventLoop,
};

pub struct App {}

#[allow(unused_variables)]
impl EventHandler<()> for App {
    fn start(&mut self, event_loop: &ActiveEventLoop<()>) {
        event_loop
            .create_window(WindowAttributes::default(), |_| {
                println!("Window created");
            })
            .unwrap();

        event_loop
            .create_window(WindowAttributes::default(), |_| {
                println!("Window created");
            })
            .unwrap();
    }

    fn suspend(&mut self, event_loop: &ActiveEventLoop<()>) {
        println!("Event loop suspended");
    }

    fn resume(&mut self, event_loop: &ActiveEventLoop<()>) {
        println!("Event loop resumed");
    }

    fn stop(&mut self) {
        println!("Event loop stopped");
    }

    fn low_memory(&mut self, event_loop: &ActiveEventLoop<()>) {
        println!("Low memory event");
    }

    fn power_source_changed(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        power_source: plinth::system::power::PowerSource,
    ) {
        println!("Power source changed: {:?}", power_source);
    }

    fn monitor_state_changed(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        monitor: plinth::system::power::MonitorState,
    ) {
        println!("Monitor state changed: {:?}", monitor);
    }

    fn power_preference_changed(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        power_preference: plinth::system::power::PowerPreference,
    ) {
        println!("Power preference changed: {:?}", power_preference);
    }

    fn activated(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window activated");
    }

    fn deactivated(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window deactivated");
    }

    fn drag_resize_started(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window drag resize started");
    }

    fn drag_resize_ended(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window drag resize ended");
    }

    fn resized(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>, size: WindowSize) {
        println!("Window resized: {:?}", size);
    }

    fn dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        dpi: DpiScale,
        size: WindowSize,
    ) {
        println!("Window DPI changed: {:?}", dpi);
        println!("Window size: {:?}", size);
    }

    fn close_requested(&mut self, _event_loop: &ActiveEventLoop<()>, mut window: Window<()>) {
        println!("Window close request");
        window.destroy();
    }

    fn shown(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window shown");
    }

    fn hidden(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window hidden");
    }

    fn maximized(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window maximized");
    }

    fn minimized(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window minimized");
    }

    fn restored(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window restored");
    }

    fn moved(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        position: WindowPoint,
    ) {
        println!("Window moved: {:?}", position);
    }

    fn wake_requested(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Window wake requested");
    }

    fn needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        reason: plinth::system::window::PaintReason,
    ) {
        println!("Window needs repaint: {:?}", reason);
    }

    fn destroyed(&mut self, _event_loop: &ActiveEventLoop<()>, _window_data: ()) {
        println!("Window destroyed");
    }

    fn key(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        code: KeyCode,
        state: ButtonState,
        modifiers: ModifierKeys,
    ) {
        println!("Key input: {:?} {:?} {:?}", code, state, modifiers);
    }

    fn mouse_button(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
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
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        position: WindowPoint,
    ) {
        println!("Mouse moved: {:?}", position);
    }

    fn pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        position: WindowPoint,
    ) {
        println!("Mouse entered: {:?}", position);
    }

    fn pointer_left(&mut self, event_loop: &ActiveEventLoop<()>, window: Window<()>) {
        println!("Mouse left");
    }

    fn mouse_scrolled(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: Window<()>,
        delta: f32,
        axis: ScrollAxis,
        modifiers: ModifierKeys,
    ) {
        println!("Mouse scrolled: {:?} {:?} {:?}", delta, axis, modifiers);
    }
}

pub fn main() {
    let app = App {};

    let mut event_loop = EventLoop::new().unwrap();
    event_loop.run(app).unwrap();
}
