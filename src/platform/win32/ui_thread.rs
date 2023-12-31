use std::{mem::MaybeUninit, sync::mpsc::Receiver};

use parking_lot::RwLock;
use windows::Win32::Foundation::HWND;

use crate::{
    application::AppContext,
    graphics::{Canvas, FrameInfo},
    limits::MAX_WINDOWS,
    math::Rect,
    platform::{
        dx12::{Context, Frame},
        gfx::{Context as _, Device, DrawList, SubmitId},
        WindowImpl,
    },
    time::{FramesPerSecond, Instant, SecondsPerFrame},
    util::{AcRead, BitMap32, Pad64},
    window::WindowEvent,
    Input, Window, WindowEventHandler, WindowEventHandlerConstructor, WindowSize,
};

use super::{
    swapchain::Swapchain,
    window::{Control, UiEvent, WindowState},
    AppContextImpl,
};

enum Mode {
    Idle,
    Animating,
}

struct RenderState {
    mode: Mode,
    handler: Box<dyn WindowEventHandler>,

    size: WindowSize,
    swapchain: Swapchain,
    draw_list: DrawList,
    frames_in_flight: [Frame; 2],

    submit_id: Option<SubmitId>,

    requested_refresh_rate: FramesPerSecond,
    actual_refresh_rate: FramesPerSecond,

    vblanks_per_frame: u16,

    next_scheduled_present: u64,

    is_drag_resizing: bool,
    need_repaint: bool,
}

impl RenderState {
    fn set_refresh_rate(&mut self, request: FramesPerSecond, composition_rate: FramesPerSecond) {
        if request == FramesPerSecond::ZERO {
            self.vblanks_per_frame = 0;
            self.actual_refresh_rate = FramesPerSecond::ZERO;
            self.mode = Mode::Idle;
        } else if request != self.requested_refresh_rate {
            let vblanks_per_frame = (composition_rate / request).floor().max(1.0);
            let refresh_rate = composition_rate / vblanks_per_frame;

            self.vblanks_per_frame = vblanks_per_frame as u16;
            self.actual_refresh_rate = refresh_rate;
            self.mode = Mode::Animating;
        }

        self.requested_refresh_rate = request;

        tracing::info!(
            "window: target fps = {:?}, target interval = {:?}",
            self.actual_refresh_rate,
            self.vblanks_per_frame
        );
    }
}

pub static WINDOWS: [Pad64<RwLock<Option<WindowState>>>; MAX_WINDOWS] = [
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
    Pad64(RwLock::new(None)),
];

pub fn spawn_ui_thread(
    context: AppContextImpl,
    ui_receiver: Receiver<UiEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || ui_thread(context, ui_receiver))
}

fn ui_thread(context: AppContextImpl, ui_receiver: Receiver<UiEvent>) {
    let mut graphics = context.dx12.create_context();
    let mut render_states = [(); MAX_WINDOWS].map(|_| MaybeUninit::uninit());

    // Strictly speaking, this is not necessary, but it's nice to have a bit of
    // insurance. Also used when iterating over windows when drawing.
    let mut occupancy = BitMap32::new();

    let mut vblank_counter = 0;

    fn get_state_mut<'a>(
        occupancy: &BitMap32,
        state: &'a mut [MaybeUninit<RenderState>],
        index: u32,
    ) -> &'a mut RenderState {
        debug_assert!(occupancy.is_set(index));
        unsafe { state[index as usize].assume_init_mut() }
    }

    'event_loop: loop {
        loop {
            match ui_receiver.try_recv() {
                Ok(event) => match event {
                    UiEvent::NewWindow(index, hwnd, constructor) => {
                        debug_assert!(!occupancy.is_set(index));

                        let render_state = &mut render_states[index as usize];
                        let window = &WINDOWS[index as usize];
                        on_new_window(&context, &graphics, render_state, window, hwnd, constructor);
                        occupancy.set(index, true);
                    }
                    UiEvent::DestroyWindow(index) => {
                        debug_assert!(occupancy.is_set(index));
                        on_destroy_window(
                            &mut render_states[index as usize],
                            &WINDOWS[index as usize],
                        );
                        occupancy.set(index, false);
                    }
                    UiEvent::Window(index, event) => on_window(
                        &graphics,
                        get_state_mut(&occupancy, &mut render_states, index),
                        &WINDOWS[index as usize],
                        event,
                    ),
                    UiEvent::Input(index, event) => {
                        on_input(get_state_mut(&occupancy, &mut render_states, index), event)
                    }
                    UiEvent::ControlEvent(index, event) => on_control(
                        get_state_mut(&occupancy, &mut render_states, index),
                        unsafe { WINDOWS[index as usize].write().as_mut().unwrap_unchecked() },
                        context.composition_rate(),
                        vblank_counter,
                        event,
                    ),
                    UiEvent::Shutdown => break 'event_loop,
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return;
                }
            }
        }

        draw_windows(
            &mut graphics,
            &mut render_states,
            &occupancy,
            vblank_counter,
            context.composition_rate(),
        );

        context.wait_for_main_monitor_vblank();
        vblank_counter += 1;
    }
}

fn on_new_window(
    context: &AppContextImpl,
    graphics: &Context,
    render_state: &mut MaybeUninit<RenderState>,
    window_state: &'static Pad64<RwLock<Option<WindowState>>>,
    hwnd: HWND,
    constructor: &WindowEventHandlerConstructor,
) {
    let swapchain = context.create_swapchain(hwnd);
    let frames_in_flight = [graphics.create_frame(), graphics.create_frame()];

    let handler = constructor(Window::new(WindowImpl::new(
        hwnd,
        AcRead::new(window_state),
        AppContext {
            inner: context.clone(),
        },
    )));

    *render_state = MaybeUninit::new(RenderState {
        mode: Mode::Idle,
        handler,
        swapchain,
        size: WindowSize {
            width: 0,
            height: 0,
            dpi: 0,
        },
        draw_list: DrawList::new(),
        frames_in_flight,
        submit_id: None,
        requested_refresh_rate: FramesPerSecond::default(),
        actual_refresh_rate: FramesPerSecond::default(),
        vblanks_per_frame: 1,
        next_scheduled_present: 0,
        is_drag_resizing: false,
        need_repaint: true,
    });

    window_state.write().replace(WindowState {
        size: WindowSize {
            width: 0,
            height: 0,
            dpi: 0,
        },
        is_visible: false,
        is_resizing: false,
        actual_refresh_rate: FramesPerSecond::ZERO,
        requested_refresh_rate: FramesPerSecond::ZERO,
        pointer_location: None,
    });
}

fn on_destroy_window(
    render_state: &mut MaybeUninit<RenderState>,
    window_state: &RwLock<Option<WindowState>>,
) {
    let _ = window_state.write().take();
    unsafe { render_state.assume_init_drop() };
}

fn on_input(render_state: &mut RenderState, event: Input) {
    render_state.handler.on_input(event);
}

fn on_window(
    graphics: &Context,
    render_state: &mut RenderState,
    window_state: &RwLock<Option<WindowState>>,
    event: WindowEvent,
) {
    {
        let mut window_write = window_state.write();
        let window = unsafe { window_write.as_mut().unwrap_unchecked() };

        match event {
            WindowEvent::CloseRequest => {}
            WindowEvent::Visible(is_visible) => {
                window.is_visible = is_visible;
            }
            WindowEvent::BeginResize => {
                render_state.is_drag_resizing = true;

                window.is_resizing = true;
            }
            WindowEvent::Resize(size) => {
                graphics.wait_for_idle();

                render_state.swapchain.resize(
                    size.width,
                    size.height,
                    render_state.is_drag_resizing.then_some(2.0),
                );

                render_state.size = size;
                window.size = size;
            }
            WindowEvent::EndResize => {
                render_state.is_drag_resizing = false;
                graphics.wait_for_idle();
                render_state.swapchain.resize(
                    render_state.size.width,
                    render_state.size.height,
                    None,
                );
            }
        }
    }

    render_state.handler.on_event(event);
}

fn on_control(
    render_state: &mut RenderState,
    window_state: &mut WindowState,
    composition_rate: FramesPerSecond,
    vblank_counter: u64,
    event: Control,
) {
    match event {
        Control::AnimationFreq(freq) => {
            render_state.set_refresh_rate(freq, composition_rate);

            window_state.actual_refresh_rate = render_state.actual_refresh_rate;
            window_state.requested_refresh_rate = render_state.requested_refresh_rate;

            match render_state.mode {
                Mode::Idle => render_state.next_scheduled_present = u64::MAX,
                Mode::Animating => render_state.next_scheduled_present = vblank_counter,
            }
        }
        Control::Repaint => {
            render_state.need_repaint = true;
        }
    }
}

fn draw_windows(
    graphics: &mut Context,
    render_state: &mut [MaybeUninit<RenderState>],
    occupancy: &BitMap32,
    vblank_counter: u64,
    composition_rate: FramesPerSecond,
) {
    for i in 0..MAX_WINDOWS as u32 {
        if !occupancy.is_set(i) {
            continue;
        }

        let window = unsafe { render_state[i as usize].assume_init_mut() };

        window.need_repaint |= window.next_scheduled_present <= vblank_counter;

        // this does two things:
        // 1. if the window is animating, it schedules the next present
        // 2. it allows for pre-emptive repainting if the window was resized
        //    without affecting the animation frequency
        while window.next_scheduled_present <= vblank_counter {
            window.next_scheduled_present += window.vblanks_per_frame as u64;
        }

        if window.need_repaint {
            window.need_repaint = false;

            if let Some(submit_id) = window.submit_id {
                graphics.wait(submit_id);
                window.submit_id = None;
            }

            let rect = {
                let size = window.size;
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32)
            };

            let mut canvas = Canvas::new(&mut window.draw_list, rect);

            let prev_present_time = window
                .swapchain
                .prev_present_time()
                .unwrap_or(Instant::ZERO);

            let next_present_time = {
                let now = Instant::now();
                let mut time = prev_present_time;
                let frame_time = composition_rate.frame_time();

                while time < now {
                    time += frame_time.0;
                }

                time
            };

            let instantaneous_frame_rate = {
                let delta = next_present_time - prev_present_time;
                SecondsPerFrame(delta).as_frames_per_second()
            };

            let timings = FrameInfo {
                target_frame_rate: window.actual_refresh_rate,
                instantaneous_frame_rate,
                prev_present_time,
                next_present_time,
            };

            // todo: send this to a worker thread
            window.handler.on_repaint(&mut canvas, &timings);

            let (image, image_index) = window.swapchain.get_back_buffer();
            let frame = &mut window.frames_in_flight[image_index as usize];

            // todo: split this into recording and submission and have the worker thread do the recording
            let submit_id = graphics.draw(canvas.finish(), frame, image);
            window.swapchain.present();
            window.submit_id = Some(submit_id);
        }
    }
}
