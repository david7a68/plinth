use crate::geometry::Rect;

use super::{
    color::Color,
    draw_list::{Primitive, PrimitiveFlags},
    image::Image,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RoundRect {
    pub(crate) data: Primitive,
}

impl RoundRect {
    #[must_use]
    pub fn new(rect: Rect) -> Self {
        Self {
            data: Primitive {
                xywh: rect.to_xywh(),
                uvwh: [0.0; 4],
                color: Color::WHITE.to_array_f32(),
                texture_id: 0,
                flags: PrimitiveFlags::new(),
                empty: 0,
            },
        }
    }

    pub fn with_color(&mut self, color: Color) -> &mut Self {
        self.data.color = color.to_array_f32();
        self
    }

    pub fn with_image(&mut self, image: Image) -> &mut Self {
        self.data.texture_id = image.id.raw;
        self
    }
}
