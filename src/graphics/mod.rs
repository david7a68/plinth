use std::collections::VecDeque;

use smallvec::SmallVec;
use windows::Win32::Foundation::HWND;

mod api;

use api::GraphicsCommandList;
pub use api::{GraphicsConfig, Image, ResizeOp, SubmissionId, Swapchain};

use self::api::ResourceState;

pub struct Device {
    device: api::Device,
    command_list_pool: SmallVec<[api::GraphicsCommandList; 2]>,
    graphics_in_flight: VecDeque<GraphicsInFlight>,
}

impl Device {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = api::Device::new(config);

        Self {
            device,
            command_list_pool: SmallVec::default(),
            graphics_in_flight: VecDeque::with_capacity(2),
        }
    }

    pub fn flush(&mut self) {
        self.device.wait_for_idle();
        self.recycle_graphics_in_flight();
    }

    pub fn create_swapchain(&self, window: HWND) -> Swapchain {
        Swapchain::new(&self.device, window)
    }

    pub fn create_canvas<'a>(&mut self, target: &'a Image) -> Canvas<'a> {
        self.recycle_graphics_in_flight();

        let mut command_list = self
            .command_list_pool
            .pop()
            .unwrap_or_else(|| GraphicsCommandList::new(&self.device));

        command_list.image_barrier(target, ResourceState::Present, ResourceState::RenderTarget);
        command_list.set_render_target(target);
        command_list.clear([1.0, 1.0, 1.0, 1.0]);

        Canvas {
            target,
            command_list,
        }
    }

    pub fn draw_canvas(&mut self, mut canvas: Canvas) {
        canvas.command_list.image_barrier(
            canvas.target,
            ResourceState::RenderTarget,
            ResourceState::Present,
        );
        canvas.command_list.finish();

        let submission_id = self
            .device
            .submit_graphics_command_list(&canvas.command_list);

        self.graphics_in_flight.push_back(GraphicsInFlight {
            command_list: canvas.command_list,
            submission_id,
        });
    }

    fn recycle_graphics_in_flight(&mut self) {
        let last_completed = self.device.most_recently_completed_submission();

        while let Some(in_flight) = self.graphics_in_flight.front() {
            if in_flight.submission_id > last_completed {
                break;
            }

            let mut in_flight = self.graphics_in_flight.pop_front().unwrap().command_list;
            in_flight.reset();
            self.command_list_pool.push(in_flight);
        }
    }
}

struct GraphicsInFlight {
    command_list: GraphicsCommandList,
    submission_id: SubmissionId,
}

pub struct Canvas<'a> {
    target: &'a Image,
    command_list: GraphicsCommandList,
}
