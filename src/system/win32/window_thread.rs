//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

use std::{
    collections::VecDeque,
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
};

use parking_lot::RwLock;

use crate::{
    graphics::{
        backend::{ResizeOp, SubmissionId, Swapchain},
        Canvas, DrawData, FrameInfo, FramesPerSecond, Graphics, Present, PresentStatistics,
        RefreshRate,
    },
    math::Scale,
    time::{Duration, Instant},
    window::{Window, WindowEventHandler},
};

use super::{
    application::{AppContextImpl, AppMessage},
    window::{Event, SharedState, WindowImpl},
};

/// Spawns a new thread to handle processing of window events.
///
/// Each handler thread can only handle one window and will panic if it receives
/// more than one `Event::Create` message. The lifetime of the spawned thread is
/// tied to the lifetime of the channel receiver and will automatically exit
/// when the channel is closed.
pub(super) fn spawn<W, F>(
    context: AppContextImpl,
    mut constructor: F,
    event_receiver: Receiver<Event>,
) where
    W: WindowEventHandler + 'static,
    F: FnMut(Window) -> W + Send + 'static,
{
    std::thread::spawn(move || {
        let AppContextImpl { graphics, sender } = context.clone();

        let shared_state = Arc::new(RwLock::new(SharedState::default()));

        let (hwnd, handler) = {
            let hwnd = match event_receiver.recv().unwrap() {
                Event::Create(hwnd) => hwnd,
                msg => panic!(
                    "First message must be Event::Create(hwnd). Got {:?} instead.",
                    msg
                ),
            };

            sender.send(AppMessage::WindowCreated).unwrap();
            (
                hwnd,
                constructor(Window::new(WindowImpl {
                    hwnd,
                    context: context.into(),
                    shared_state: shared_state.clone(),
                })),
            )
        };

        State::new(hwnd, handler, shared_state, &graphics).run(&event_receiver);
        sender.send(AppMessage::WindowClosed).unwrap();
    });
}

const FRAME_TIME_WINDOW_SIZE: usize = 10;

struct State<'a, W>
where
    W: WindowEventHandler + 'static,
{
    handler: W,
    shared_state: Arc<RwLock<SharedState>>,

    graphics: &'a Arc<Graphics>,
    swapchain: Swapchain,
    draw_data: DrawData,

    repaint_clock: RepaintClock,

    submission_id: Option<SubmissionId>,
    is_drag_resizing: bool,
    need_repaint: bool,
    repaint_now: bool,
}

impl<'a, W> State<'a, W>
where
    W: WindowEventHandler + 'static,
{
    fn new(
        hwnd: windows::Win32::Foundation::HWND,
        handler: W,
        shared_state: Arc<RwLock<SharedState>>,
        graphics: &'a Arc<Graphics>,
    ) -> Self {
        let swapchain = graphics.create_swapchain(hwnd);
        let draw_data = graphics.create_draw_buffer();

        let present_info = swapchain.present_statistics();
        let repaint_clock = RepaintClock::new(present_info.monitor_rate);

        shared_state.write().refresh_rate = RefreshRate {
            min_fps: FramesPerSecond(0.0),
            max_fps: present_info.monitor_rate,
            optimal_fps: repaint_clock.effective_refresh_rate,
        };

        Self {
            handler,
            shared_state,
            graphics,
            swapchain,
            draw_data,
            repaint_clock,
            submission_id: None,
            is_drag_resizing: false,
            need_repaint: false,
            repaint_now: false,
        }
    }

    fn run(&mut self, event_receiver: &Receiver<Event>) {
        while self.process_pending::<false>(event_receiver) {
            if self.repaint_clock.is_running() && self.shared_state.read().is_visible {
                // use OR so that we don't overwrite a repaint request from the event loop
                if self.repaint_clock.should_repaint() {
                    self.need_repaint = true;
                } else {
                    self.swapchain.wait_for_vsync();
                }
            } else {
                self.process_pending::<true>(event_receiver);
            }

            // repaint after processing inputs
            if self.need_repaint {
                let start = Instant::now();
                self.repaint();
                let elapsed = start.elapsed();

                self.swapchain.wait_for_vsync();

                let present_info = self.swapchain.present_statistics();

                self.repaint_clock.update(&present_info, &elapsed);
                self.shared_state.write().refresh_rate = RefreshRate {
                    min_fps: FramesPerSecond(0.0),
                    max_fps: present_info.monitor_rate,
                    optimal_fps: self.repaint_clock.effective_refresh_rate,
                };

                #[cfg(feature = "profile")]
                {
                    tracing_tracy::client::frame_mark();
                }
            }
        }
    }

    /// Returns `true` if the window is still open, `false` if it has been
    /// destroyed.
    #[tracing::instrument(skip(self, event_receiver))]
    fn process_pending<const BLOCK: bool>(&mut self, event_receiver: &Receiver<Event>) -> bool {
        if BLOCK {
            // only fails if the channel is closed
            let Ok(event) = event_receiver.recv() else {
                return false;
            };

            // This is so that we don't try to render after returning, even
            // though the channel is still open.
            if !self.on_event(event) {
                return false;
            }
        }

        loop {
            match event_receiver.try_recv() {
                Ok(event) => {
                    if !self.on_event(event) {
                        break false;
                    }
                }
                Err(e) => match e {
                    TryRecvError::Empty => break true,
                    TryRecvError::Disconnected => break false,
                },
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn on_event(&mut self, event: Event) -> bool {
        // default return true, explicitly return false if we want to exit
        match event {
            Event::Create(_) => {
                panic!("Window already created");
            }
            Event::CloseRequest => {
                self.handler.on_close_request();
            }
            Event::Destroy => {
                self.handler.on_destroy();
                return false;
            }
            Event::Visible(is_visible) => {
                self.handler.on_visible(is_visible);
            }
            Event::BeginResize => {
                self.is_drag_resizing = true;
                self.handler.on_begin_resize();
            }
            Event::Resize {
                width,
                height,
                scale,
            } => {
                let op = if self.is_drag_resizing {
                    ResizeOp::Flex {
                        width,
                        height,
                        flex: 2.0,
                    }
                } else {
                    ResizeOp::Fixed { width, height }
                };

                self.graphics.resize_swapchain(&mut self.swapchain, op);

                let size = (width, height).into();

                {
                    let mut state = self.shared_state.write();
                    state.size = size;
                    state.is_visible = width > 0 && height > 0;
                }

                self.handler.on_resize(size, Scale::new(scale, scale));
            }
            Event::EndResize => {
                self.is_drag_resizing = false;

                let size = self.shared_state.read().size;
                self.graphics.resize_swapchain(
                    &mut self.swapchain,
                    ResizeOp::Fixed {
                        width: size.width as u32,
                        height: size.height as u32,
                    },
                );

                self.handler.on_end_resize();
            }
            Event::SetAnimationFrequency(freq) => {
                self.repaint_clock.requested_refresh_rate(freq);
            }
            Event::Repaint => {
                self.need_repaint = true;
                self.repaint_now = true;
            }
            Event::PointerMove(location) => {
                let location = location.into();
                let delta = {
                    let mut shared_state = self.shared_state.write();
                    shared_state.pointer_location = Some(location);

                    if let Some(last_cursor_pos) = shared_state.pointer_location {
                        location - last_cursor_pos
                    } else {
                        (0.0, 0.0).into()
                    }
                };

                self.handler.on_pointer_move(location, delta.into());
            }
            Event::PointerLeave => {
                self.handler.on_pointer_leave();
            }
            Event::MouseButton(button, state, location) => {
                self.handler.on_mouse_button(button, state, location.into());
            }
            Event::Scroll(axis, delta) => {
                self.handler.on_scroll(axis, delta);
            }
        }

        true
    }

    #[tracing::instrument(skip(self))]
    fn repaint(&mut self) {
        let (image, _) = self.swapchain.get_back_buffer();

        if let Some(submission_id) = self.submission_id {
            self.graphics.wait_for_submission(submission_id);
        }

        let stats = self.repaint_clock.next_present_info();

        let draw_data = {
            self.draw_data.reset();
            let rect = self.shared_state.read().size.into();

            let mut canvas = Canvas::new(&mut self.draw_data, rect, image);
            self.handler.on_repaint(&mut canvas, &stats);
            canvas.finish()
        };

        draw_data.finish();
        // copy geometry from the geometry buffer to a temp buffer and
        let submit_id = self.graphics.draw(draw_data);

        self.submission_id = Some(submit_id);

        let interval = if self.repaint_now {
            0
        } else {
            self.repaint_clock.derived_intervals_per_frame
        };

        self.swapchain.present(submit_id, interval);

        self.need_repaint = false;
        self.repaint_now = false;
    }
}

impl<W> Drop for State<'_, W>
where
    W: WindowEventHandler + 'static,
{
    fn drop(&mut self) {
        self.graphics.wait_for_idle();
    }
}

/// Keeps track of presentation statistics and determines when to repaint.
///
/// This is mostly just to consolidate timing logic into one place.
#[derive(Debug)]
struct RepaintClock {
    /// The refresh rate of the window. This may be different from the actual
    /// monitor refresh rate if the window compositor is not synchronized to the
    /// monitor.
    ///
    /// For example, the Windows compositor is synced to the primary monitor and
    /// everything else just has to make do.
    monitor_refresh_rate: FramesPerSecond,

    /// The period between monitor refreshes.
    monitor_refresh_time: Duration,

    /// The refresh rate requested by the application.
    requested_refresh_rate: FramesPerSecond,

    /// The actual refresh rate that the application will get. This will be
    /// faster than the requested refresh rate unless the requested refresh rate
    /// exceeds the monitor refresh rate.
    effective_refresh_rate: FramesPerSecond,

    /// The period between frames at the effective refresh rate.
    derived_frame_budget: Duration,
    derived_intervals_per_frame: u32,

    prev_present_time: Instant,
    next_target_present_time: Instant,

    historical_frame_times: VecDeque<Duration>,
    mean_frame_time: Duration,

    min_presentation_time: Duration,

    prev_present_id: u64,
}

impl RepaintClock {
    fn new(monitor_refresh_rate: FramesPerSecond) -> Self {
        Self {
            monitor_refresh_rate,
            monitor_refresh_time: monitor_refresh_rate.frame_time().0,
            requested_refresh_rate: monitor_refresh_rate,
            effective_refresh_rate: monitor_refresh_rate,
            derived_frame_budget: monitor_refresh_rate.frame_time().0,
            derived_intervals_per_frame: 1,
            prev_present_time: Instant::ZERO,
            next_target_present_time: Instant::ZERO,
            historical_frame_times: VecDeque::new(),
            mean_frame_time: Duration::ZERO,
            min_presentation_time: Duration::ZERO,
            prev_present_id: 0,
        }
    }

    fn is_running(&self) -> bool {
        self.requested_refresh_rate.0 > 0.0
    }

    fn requested_refresh_rate(&mut self, rate: FramesPerSecond) {
        self.requested_refresh_rate = rate;
    }

    fn next_present_info(&self) -> FrameInfo {
        // When we estimate rendering will complete if we draw right now based
        // on the previous frame.
        let estimated_render_complete_time = Instant::now() + self.mean_frame_time;

        assert!(
            estimated_render_complete_time >= self.prev_present_time,
            "estimated_render_complete_time: {:?}, self.prev_present_time: {:?}",
            estimated_render_complete_time,
            self.prev_present_time
        );

        // The estimated time until we are able to present the next frame
        // (rendering is complete).
        let estimated_time_until_present = estimated_render_complete_time - self.prev_present_time;

        let estimated_present_time = estimated_render_complete_time + self.min_presentation_time;

        // The number of monitor refreshes until we are next able to present,
        // adjusted to the next vsync.
        let estimated_intervals = self
            .monitor_refresh_time
            .frames_for(estimated_time_until_present);

        assert!(
            estimated_present_time >= estimated_render_complete_time,
            "estimated_present_time: {:?}, self.prev_present_time: {:?}",
            estimated_present_time,
            self.prev_present_time
        );

        let prev_present = Present {
            id: self.prev_present_id,
            time: self.prev_present_time,
        };

        let next_present = Present {
            id: self.prev_present_id + estimated_intervals.num_frames as u64,
            time: estimated_present_time,
        };

        FrameInfo {
            prev_present,
            next_present,
        }
    }

    fn update(&mut self, stats: &PresentStatistics, cpu_time: &Duration) {
        #[cfg(feature = "profile")]
        {
            // plot delta between expected and actual present time
            tracing_tracy::client::plot!(
                "delta between expected and actual present time",
                (stats.prev_present_time - self.next_target_present_time).0
                    / self.monitor_refresh_time
            );
        }

        // This may change every frame, depending on the compositor.
        if self.monitor_refresh_rate != stats.monitor_rate {
            self.monitor_refresh_rate = stats.monitor_rate;
            self.monitor_refresh_time = stats.monitor_rate.frame_time().0;

            if self.requested_refresh_rate > 0.0 {
                // This can be zero, meaning that we want to draw every vsync.
                let vblanks_per_frame = (stats.monitor_rate.round() / self.requested_refresh_rate)
                    .floor()
                    .max(1.0);

                self.derived_intervals_per_frame = vblanks_per_frame as u32;
                self.effective_refresh_rate = stats.monitor_rate.round() / vblanks_per_frame;
                self.derived_frame_budget = (self.monitor_refresh_time * vblanks_per_frame).into();
            }
        }

        self.historical_frame_times.push_back(*cpu_time);
        if self.historical_frame_times.len() > FRAME_TIME_WINDOW_SIZE {
            self.historical_frame_times.pop_front();
        }

        self.mean_frame_time = self.historical_frame_times.iter().sum::<Duration>()
            / self.historical_frame_times.len() as f64;

        self.prev_present_time = stats.prev_present_time;
        self.next_target_present_time = stats.prev_present_time + self.derived_frame_budget;
        self.prev_present_id += 1;

        #[cfg(feature = "profile")]
        {}
    }

    /// Tries to determine when it makes sense to repaint.
    ///
    /// The later we draw, the less input latency we have, but the more likely
    /// we are to miss the presentation window. So, we want to draw such that we
    /// expect to be finished within one monitor refresh of the target vblank.
    ///
    /// This will not save us if the estimated render time is longer than the
    /// refresh interval.
    fn should_repaint(&self) -> bool {
        let now = Instant::now();

        #[cfg(feature = "profile")]
        {
            tracing_tracy::client::plot!(
                "time until present",
                (self.next_target_present_time - now).0 / self.monitor_refresh_time
            );
        }

        if self.prev_present_time > now {
            // The previously submitted frame is still in flight.
            return false;
        } else if now >= self.next_target_present_time || (self.derived_intervals_per_frame == 1) {
            // We missed the presentation window.x
            return true;
        }

        // mean frame time, rounded to the next vsync
        let mean_frame_time =
            (self.mean_frame_time / self.monitor_refresh_time).ceil() * self.monitor_refresh_time;

        (self.next_target_present_time - mean_frame_time - now) <= self.monitor_refresh_time
    }
}
