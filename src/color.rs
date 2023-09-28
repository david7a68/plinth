use std::marker::PhantomData;

pub trait ColorSpace {}

pub struct Srgb;

impl ColorSpace for Srgb {}

pub struct AnyColorSpace;

impl ColorSpace for AnyColorSpace {}

///
pub struct Color<CS: ColorSpace = AnyColorSpace> {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    color_space: PhantomData<CS>,
}

impl<CS: ColorSpace> Clone for Color<CS> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<CS: ColorSpace> Copy for Color<CS> {}

impl<CS: ColorSpace> Color<CS> {
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
        color_space: PhantomData,
    };
}
