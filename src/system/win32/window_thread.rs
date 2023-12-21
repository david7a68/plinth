//! Window thread.
//!
//! Each window runs on its own thread, receiving events from a channel.

use std::{
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
    time::Duration,
};

use parking_lot::RwLock;

use crate::{
    graphics::{
        backend::{ResizeOp, SubmissionId, Swapchain},
        Canvas, DrawData, FrameInfo, FramesPerSecond, Graphics, Present, PresentInstant,
        PresentStatistics, RefreshRate, SecondsPerFrame,
    },
    math::Scale,
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
    last_frame_time: Duration,

    submission_id: Option<SubmissionId>,
    is_drag_resizing: bool,
    need_repaint: bool,
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

        let present_statistics = swapchain.present_statistics();
        let repaint_clock = RepaintClock::new(present_statistics.monitor_rate);

        Self {
            handler,
            shared_state,
            graphics,
            swapchain,
            draw_data,
            repaint_clock,
            last_frame_time: Duration::ZERO,
            submission_id: None,
            is_drag_resizing: false,
            need_repaint: false,
        }
    }

    fn run(&mut self, event_receiver: &Receiver<Event>) {
        while self.process_pending::<false>(event_receiver) {
            if self.repaint_clock.is_running() && self.shared_state.read().is_visible {
                // use OR so that we don't overwrite a repaint request from the event loop
                self.need_repaint |= self.repaint_clock.should_repaint(self.last_frame_time);
            } else {
                self.process_pending::<true>(event_receiver);
            }

            // repaint after processing inputs
            if self.need_repaint {
                let start = PresentInstant::now();
                self.repaint();
                self.last_frame_time = start.elapsed();

                self.swapchain.wait_for_vsync();

                let present_info = self.swapchain.present_statistics();
                self.repaint_clock.update(&present_info);
                self.shared_state.write().refresh_rate = RefreshRate {
                    min_fps: FramesPerSecond(0.0),
                    max_fps: present_info.monitor_rate,
                    optimal_fps: self.repaint_clock.effective_refresh_rate,
                };
            }
        }
    }

    /// Returns `true` if the window is still open, `false` if it has been
    /// destroyed.
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

    fn repaint(&mut self) {
        let (image, _) = self.swapchain.get_back_buffer();

        if let Some(submission_id) = self.submission_id {
            self.graphics.wait_for_submission(submission_id);
        }

        let stats = self.repaint_clock.next_present_info(self.last_frame_time);

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
        self.swapchain.present(submit_id);

        self.need_repaint = false;
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
    monitor_refresh_time: SecondsPerFrame,

    /// The refresh rate requested by the application.
    requested_refresh_rate: FramesPerSecond,

    /// The actual refresh rate that the application will get. This will be
    /// faster than the requested refresh rate unless the requested refresh rate
    /// exceeds the monitor refresh rate.
    effective_refresh_rate: FramesPerSecond,

    /// The period between frames at the effective refresh rate.
    effective_refresh_time: SecondsPerFrame,

    prev_present_time: PresentInstant,
    next_target_present_time: PresentInstant,

    prev_present_id: u64,
}

impl RepaintClock {
    fn new(monitor_refresh_rate: FramesPerSecond) -> Self {
        Self {
            monitor_refresh_rate,
            monitor_refresh_time: SecondsPerFrame::from(monitor_refresh_rate),
            requested_refresh_rate: FramesPerSecond::ZERO,
            effective_refresh_rate: FramesPerSecond::ZERO,
            effective_refresh_time: SecondsPerFrame::ZERO,
            prev_present_time: PresentInstant::ZERO,
            next_target_present_time: PresentInstant::ZERO,
            prev_present_id: 0,
        }
    }

    fn is_running(&self) -> bool {
        self.requested_refresh_rate.0 > 0.0
    }

    fn requested_refresh_rate(&mut self, rate: FramesPerSecond) {
        self.requested_refresh_rate = rate;
    }

    fn next_present_info(&self, estimated_render_time: Duration) -> FrameInfo {
        // When we estimate rendering will complete if we draw right now based
        // on the previous frame.
        let estimated_render_complete_time = PresentInstant::now() + estimated_render_time;

        tracing::info!("estimated_render_time: {:?}", estimated_render_time);

        tracing::info!(
            "estimated_render_complete_time: {:?}",
            estimated_render_complete_time
        );

        tracing::info!(
            "next_target_present_time: {:?}",
            self.next_target_present_time
        );

        // The estimated time until we are able to present the next frame
        // (rendering is complete).
        let estimated_time_until_present = estimated_render_complete_time - self.prev_present_time;

        tracing::info!(
            "estimated_time_until_present: {:?}",
            estimated_time_until_present
        );

        // The number of monitor refreshes until we are next able to present,
        // adjusted to the next vsync.
        let estimated_intervals = self
            .monitor_refresh_time
            .interval_over(estimated_time_until_present);

        tracing::info!("estimated_intervals: {:?}", estimated_intervals);

        tracing::info!("prev_present_time: {:?}", self.prev_present_time);

        let estimated_present_time = self.prev_present_time + estimated_intervals.time;

        tracing::info!("estimated_present_time: {:?}", estimated_present_time);

        FrameInfo {
            prev_present: Present {
                id: self.prev_present_id,
                time: self.prev_present_time,
            },
            next_present: Present {
                id: self.prev_present_id + 1,
                time: estimated_present_time,
            },
        }
    }

    fn update(&mut self, stats: &PresentStatistics) {
        // This may change every frame, depending on the compositor.
        if self.monitor_refresh_rate != stats.monitor_rate {
            self.monitor_refresh_rate = stats.monitor_rate;
            self.monitor_refresh_time = SecondsPerFrame::from(stats.monitor_rate);

            if self.requested_refresh_rate > 0.0 {
                // This can be zero, meaning that we want to draw every vsync.
                let vblanks_per_frame = (stats.monitor_rate / self.requested_refresh_rate)
                    .floor()
                    .max(1.0);

                self.effective_refresh_rate = stats.monitor_rate / vblanks_per_frame;
                self.effective_refresh_time = self.monitor_refresh_time * vblanks_per_frame;
            }
        }

        self.prev_present_time = stats.prev_present_time;
        self.next_target_present_time = stats.prev_present_time + self.effective_refresh_time;
        self.prev_present_id += 1;
    }

    /// Tries to determine when it makes sense to repaint.
    ///
    /// The later we draw, the less input latency we have, but the more likely
    /// we are to miss the presentation window. So, we want to draw such that we
    /// expect to be finished within one monitor refresh of the target vblank.
    ///
    /// This will not save us if the estimated render time is longer than the
    /// refresh interval.
    fn should_repaint(&self, estimated_render_time: Duration) -> bool {
        // When we think rendering will complete if we draw right now.
        let estimated_render_complete_time = PresentInstant::now() + estimated_render_time;

        // Return true if rendering now will place us within one _monitor_
        // refresh of the target present time.
        estimated_render_complete_time + self.monitor_refresh_time >= self.next_target_present_time
    }
}
