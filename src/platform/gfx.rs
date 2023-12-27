use crate::graphics::{Color, RoundRect};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubmitId(pub u64);

pub enum DrawCommand {
    Begin,
    End,
    Clear(Color),
    DrawRects { first: u32, count: u32 },
}

pub struct DrawList {
    pub(crate) rects: Vec<RoundRect<()>>,
    pub(crate) commands: Vec<DrawCommand>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            rects: Vec::new(),
            commands: Vec::new(),
        }
    }
}

pub trait Frame {}

pub trait Image {}

pub trait RenderTarget: Image {}

pub trait Device {
    type Frame: Frame;
    type Image: Image;

    fn create_frame(&self) -> Self::Frame;

    fn draw(
        &self,
        content: &DrawList,
        frame: &mut Self::Frame,
        image: impl Into<Self::Image>,
    ) -> SubmitId;

    fn wait(&self, submit_id: SubmitId);

    fn wait_for_idle(&self);
}
