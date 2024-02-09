use plinth::system::{
    event_loop::{ActiveEventLoop, EventHandler, EventLoop},
    input::{ModifierKeys, ScrollAxis},
    window::{PhysicalPosition, PhysicalSize, Window, WindowAttributes},
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

    fn window_activated(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window activated");
    }

    fn window_deactivated(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window deactivated");
    }

    fn window_drag_resize_started(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
    ) {
        println!("Window drag resize started");
    }

    fn window_drag_resize_ended(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
    ) {
        println!("Window drag resize ended");
    }

    fn window_resized(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        size: PhysicalSize,
    ) {
        println!("Window resized: {:?}", size);
    }

    fn window_dpi_changed(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        dpi: plinth::system::window::DpiScale,
        size: plinth::system::window::PhysicalSize,
    ) {
        println!("Window DPI changed: {:?}", dpi);
        println!("Window size: {:?}", size);
    }

    fn window_close_requested(
        &mut self,
        _event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
    ) {
        println!("Window close request");
        window.destroy();
    }

    fn window_shown(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window shown");
    }

    fn window_hidden(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window hidden");
    }

    fn window_maximized(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window maximized");
    }

    fn window_minimized(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window minimized");
    }

    fn window_restored(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window restored");
    }

    fn window_moved(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        position: PhysicalPosition,
    ) {
        println!("Window moved: {:?}", position);
    }

    fn window_wake_requested(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Window wake requested");
    }

    fn window_needs_repaint(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        reason: plinth::system::window::PaintReason,
    ) {
        println!("Window needs repaint: {:?}", reason);
    }

    fn window_destroyed(&mut self, _event_loop: &ActiveEventLoop<()>, _window_data: ()) {
        println!("Window destroyed");
    }

    fn input_key(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        code: plinth::system::input::KeyCode,
        state: plinth::system::input::ButtonState,
        modifiers: plinth::system::input::ModifierKeys,
    ) {
        println!("Key input: {:?} {:?} {:?}", code, state, modifiers);
    }

    fn input_mouse_button(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        button: plinth::system::input::MouseButton,
        state: plinth::system::input::ButtonState,
        position: plinth::system::window::PhysicalPosition,
        modifiers: plinth::system::input::ModifierKeys,
    ) {
        println!(
            "Mouse button input: {:?} {:?} {:?} {:?}",
            button, state, position, modifiers
        );
    }

    fn input_pointer_move(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        position: PhysicalPosition,
    ) {
        println!("Mouse moved: {:?}", position);
    }

    fn input_pointer_entered(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
        position: PhysicalPosition,
    ) {
        println!("Mouse entered: {:?}", position);
    }

    fn input_pointer_leave(&mut self, event_loop: &ActiveEventLoop<()>, window: &mut Window<()>) {
        println!("Mouse left");
    }

    fn input_scroll(
        &mut self,
        event_loop: &ActiveEventLoop<()>,
        window: &mut Window<()>,
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
