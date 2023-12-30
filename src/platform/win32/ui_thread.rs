use std::{
    mem::MaybeUninit,
    sync::{atomic::Ordering, mpsc::Receiver},
};

use crate::{
    graphics::{Canvas, FrameInfo},
    limits::MAX_WINDOWS,
    math::Rect,
    platform::{
        dx12::{Context, Frame},
        gfx::{Context as _, Device, DrawList, SubmitId},
    },
    time::{FramesPerSecond, Instant, SecondsPerFrame},
    window::WindowEvent,
    WindowEventHandler,
};

use super::{
    swapchain::Swapchain,
    window::{Control, UiEvent, WindowOccupancy, WINDOWS},
    AppContextImpl,
};

enum Mode {
    Idle,
    Animating,
}

struct RenderState {
    mode: Mode,
    handler: Box<dyn WindowEventHandler>,

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

pub fn spawn_ui_thread(
    context: AppContextImpl,
    ui_receiver: Receiver<UiEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || ui_thread(context, ui_receiver))
}

fn ui_thread(context: AppContextImpl, ui_receiver: Receiver<UiEvent>) {
    let mut ui_thread = UiThread {
        graphics: context.dx12.create_context(),
        context,
        windows: [(); MAX_WINDOWS].map(|_| MaybeUninit::uninit()),
        occupancy: WindowOccupancy::new(),
        vblank_counter: 0,
    };

    'event_loop: loop {
        loop {
            match ui_receiver.try_recv() {
                Ok(event) => {
                    if !ui_thread.on_event(event) {
                        break 'event_loop;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return;
                }
            }
        }

        ui_thread.draw_windows();
        ui_thread.next_vblank();
    }

    // todo: cleanup??
}

struct UiThread {
    context: AppContextImpl,
    graphics: Context,
    windows: [MaybeUninit<RenderState>; MAX_WINDOWS],
    occupancy: WindowOccupancy,
    vblank_counter: u64,
}

impl UiThread {
    fn on_event(&mut self, event: UiEvent) -> bool {
        match event {
            UiEvent::NewWindow(index, hwnd, handler) => {
                debug_assert!(!self.occupancy.is_occupied(index));

                let swapchain = self.context.create_swapchain(hwnd);
                let frames_in_flight = [self.graphics.create_frame(), self.graphics.create_frame()];

                self.windows[index as usize] = MaybeUninit::new(RenderState {
                    mode: Mode::Idle,
                    handler,
                    swapchain,
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

                self.occupancy.set_occupied(index, true);
            }
            UiEvent::Shutdown => {
                return false;
            }
            UiEvent::Input(index, event) => {
                // all shared input states updates in event

                debug_assert!(self.occupancy.is_occupied(index));
                unsafe { self.windows[index as usize].assume_init_mut() }
                    .handler
                    .on_input(event);
            }
            UiEvent::Window(index, event) => {
                debug_assert!(self.occupancy.is_occupied(index));
                let window = unsafe { self.windows[index as usize].assume_init_mut() };

                let mut destroy = false;

                match event {
                    WindowEvent::CloseRequest => {}
                    WindowEvent::Destroy => {
                        destroy = true;
                    }
                    WindowEvent::Visible(is_visible) => {
                        WINDOWS[index as usize]
                            .is_visible
                            .store(is_visible, Ordering::Release);
                    }
                    WindowEvent::BeginResize => {
                        window.is_drag_resizing = true;
                    }
                    WindowEvent::Resize(size) => {
                        let flex = window.is_drag_resizing.then_some(2.0);

                        self.graphics.wait_for_idle();
                        window.swapchain.resize(size.width, size.height, flex);

                        *WINDOWS[index as usize].size.write() = size;
                    }
                    WindowEvent::EndResize => {
                        window.is_drag_resizing = false;

                        let size = WINDOWS[index as usize].size.read().clone();

                        self.graphics.wait_for_idle();
                        window.swapchain.resize(size.width, size.height, None);
                    }
                }

                window.handler.on_event(event);

                if destroy {
                    self.occupancy.set_occupied(index, false);
                    unsafe { self.windows[index as usize].assume_init_drop() };
                }
            }
            UiEvent::ControlEvent(index, event) => match event {
                Control::AnimationFreq(freq) => {
                    debug_assert!(self.occupancy.is_occupied(index));
                    let window = unsafe { self.windows[index as usize].assume_init_mut() };
                    window.set_refresh_rate(freq, self.context.composition_rate());

                    match window.mode {
                        Mode::Idle => window.next_scheduled_present = u64::MAX,
                        Mode::Animating => window.next_scheduled_present = self.vblank_counter,
                    }
                }
                Control::Repaint => {
                    debug_assert!(self.occupancy.is_occupied(index));
                    let window = unsafe { self.windows[index as usize].assume_init_mut() };
                    window.need_repaint = true;
                }
            },
        }

        true
    }

    fn draw_windows(&mut self) {
        for i in 0..MAX_WINDOWS as u32 {
            if !self.occupancy.is_occupied(i) {
                continue;
            }

            let window = unsafe { self.windows[i as usize].assume_init_mut() };

            window.need_repaint |= window.next_scheduled_present <= self.vblank_counter;

            // this does two things:
            // 1. if the window is animating, it schedules the next present
            // 2. it allows for pre-emptive repainting if the window was resized
            //    without affecting the animation frequency
            while window.next_scheduled_present <= self.vblank_counter {
                window.next_scheduled_present += window.vblanks_per_frame as u64;
            }

            if window.need_repaint {
                window.need_repaint = false;

                if let Some(submit_id) = window.submit_id {
                    self.graphics.wait(submit_id);
                    window.submit_id = None;
                }

                let rect = {
                    let size = WINDOWS[i as usize].size.read();
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
                    let frame_time = self.context.composition_rate().frame_time();

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
                let submit_id = self.graphics.draw(canvas.finish(), frame, image);
                window.swapchain.present();
                window.submit_id = Some(submit_id);
            }
        }
    }

    fn next_vblank(&mut self) {
        self.context.wait_for_main_monitor_vblank();
        self.vblank_counter += 1;
    }
}
