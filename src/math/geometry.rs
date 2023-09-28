use std::marker::PhantomData;

use super::unit::CoordinateUnit;

macro_rules! vec_like {
    ($struct_name:ident, $x_component:ident, $y_component:ident) => {
        pub struct $struct_name<U: CoordinateUnit> {
            pub $x_component: f32,
            pub $y_component: f32,
            coordinate_space: PhantomData<U>,
        }

        impl<U: CoordinateUnit> $struct_name<U> {
            pub const ZERO: Self = Self::new(0.0, 0.0);
            pub const ONE: Self = Self::new(1.0, 1.0);

            pub const fn new($x_component: f32, $y_component: f32) -> Self {
                Self {
                    $x_component,
                    $y_component,
                    coordinate_space: PhantomData,
                }
            }

            pub fn cast<OtherCS: CoordinateUnit + From<U>>(self) -> $struct_name<OtherCS> {
                $struct_name {
                    $x_component: self.$x_component,
                    $y_component: self.$y_component,
                    coordinate_space: PhantomData,
                }
            }
        }

        impl<U: CoordinateUnit> Clone for $struct_name<U> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<U: CoordinateUnit> Copy for $struct_name<U> {}

        impl<U: CoordinateUnit> std::ops::Neg for $struct_name<U> {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self {
                    $x_component: -self.$x_component,
                    $y_component: -self.$y_component,
                    coordinate_space: PhantomData,
                }
            }
        }
    };
}

macro_rules! vec_impl_scale {
    ($lhs:ident, $lhs_x:ident, $lhs_y:ident) => {
        impl<U: CoordinateUnit> $lhs<U> {
            pub fn scale_by<U2: CoordinateUnit>(&self, scale: Scale2D<U, U2>) -> $lhs<U2> {
                $lhs {
                    $lhs_x: self.$lhs_x * scale.x,
                    $lhs_y: self.$lhs_y * scale.y,
                    coordinate_space: PhantomData,
                }
            }
        }
    };
}

macro_rules! vec_impl_add_sub {
    ($lhs:ident, $lhs_x:ident, $lhs_y:ident, $rhs:ident, $rhs_x:ident, $rhs_y:ident, $out:ident, $out_x:ident, $out_y:ident) => {
        impl<U: CoordinateUnit> std::ops::Add<$rhs<U>> for $lhs<U> {
            type Output = $out<U>;

            fn add(self, rhs: $rhs<U>) -> Self::Output {
                Self::Output {
                    $out_x: self.$lhs_x + rhs.$rhs_x,
                    $out_y: self.$lhs_y + rhs.$rhs_y,
                    coordinate_space: PhantomData,
                }
            }
        }

        impl<U: CoordinateUnit> std::ops::Sub<$rhs<U>> for $lhs<U> {
            type Output = $out<U>;

            fn sub(self, rhs: $rhs<U>) -> Self::Output {
                Self::Output {
                    $out_x: self.$lhs_x - rhs.$rhs_x,
                    $out_y: self.$lhs_y - rhs.$rhs_y,
                    coordinate_space: PhantomData,
                }
            }
        }
    };
}

macro_rules! vec_impl_add_sub_self {
    ($lhs:ident, $lhs_x:ident, $lhs_y:ident, $rhs:ident, $rhs_x:ident, $rhs_y:ident) => {
        impl<U: CoordinateUnit> std::ops::AddAssign<$rhs<U>> for $lhs<U> {
            fn add_assign(&mut self, rhs: $rhs<U>) {
                *self = *self + rhs;
            }
        }

        impl<U: CoordinateUnit> std::ops::SubAssign<$rhs<U>> for $lhs<U> {
            fn sub_assign(&mut self, rhs: $rhs<U>) {
                *self = *self - rhs;
            }
        }
    };
}

macro_rules! vec_imp_scalar_mul_div {
    ($lhs:ident, $lhs_x:ident, $lhs_y:ident) => {
        impl<U: CoordinateUnit> std::ops::Mul<f32> for $lhs<U> {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self::Output {
                Self {
                    $lhs_x: self.$lhs_x * rhs,
                    $lhs_y: self.$lhs_y * rhs,
                    coordinate_space: PhantomData,
                }
            }
        }

        impl<U: CoordinateUnit> std::ops::MulAssign<f32> for $lhs<U> {
            fn mul_assign(&mut self, rhs: f32) {
                *self = *self * rhs;
            }
        }

        impl<U: CoordinateUnit> std::ops::Div<f32> for $lhs<U> {
            type Output = Self;

            fn div(self, rhs: f32) -> Self::Output {
                Self {
                    $lhs_x: self.$lhs_x / rhs,
                    $lhs_y: self.$lhs_y / rhs,
                    coordinate_space: PhantomData,
                }
            }
        }

        impl<U: CoordinateUnit> std::ops::DivAssign<f32> for $lhs<U> {
            fn div_assign(&mut self, rhs: f32) {
                *self = *self / rhs;
            }
        }
    };
}

vec_like!(Point2D, x, y);
vec_impl_scale!(Point2D, x, y);
vec_impl_add_sub!(Point2D, x, y, Translate2D, x, y, Point2D, x, y);
vec_impl_add_sub!(Point2D, x, y, Point2D, x, y, Translate2D, x, y);

vec_like!(Translate2D, x, y);
vec_impl_scale!(Translate2D, x, y);
vec_impl_add_sub!(Translate2D, x, y, Translate2D, x, y, Translate2D, x, y);
vec_impl_add_sub!(Translate2D, x, y, Point2D, x, y, Point2D, x, y);
vec_impl_add_sub_self!(Translate2D, x, y, Translate2D, x, y);
vec_imp_scalar_mul_div!(Translate2D, x, y);

impl<U: CoordinateUnit> From<Size2D<U>> for Translate2D<U> {
    fn from(size: Size2D<U>) -> Self {
        Self::new(size.width, size.height)
    }
}

vec_like!(Size2D, width, height);
vec_imp_scalar_mul_div!(Size2D, width, height);

pub struct Scale2D<Src: CoordinateUnit, Dst: CoordinateUnit> {
    pub x: f32,
    pub y: f32,
    coordinate_space: PhantomData<(Src, Dst)>,
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> Clone for Scale2D<Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> Copy for Scale2D<Src, Dst> {}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> Scale2D<Src, Dst> {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            coordinate_space: PhantomData,
        }
    }
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> std::ops::Neg for Scale2D<Src, Dst> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            coordinate_space: PhantomData,
        }
    }
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> std::ops::Mul<f32> for Scale2D<Src, Dst> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            coordinate_space: PhantomData,
        }
    }
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> std::ops::MulAssign<f32> for Scale2D<Src, Dst> {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl<Src: CoordinateUnit, Dst: CoordinateUnit> std::ops::Div<f32> for Scale2D<Src, Dst> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            coordinate_space: PhantomData,
        }
    }
}

impl<U> From<Size2D<U>> for Rect2D<U>
where
    U: CoordinateUnit,
{
    fn from(size: Size2D<U>) -> Self {
        Self::from_size(size)
    }
}

pub struct Rect2D<U: CoordinateUnit> {
    pub top_left: Point2D<U>,
    pub bottom_right: Point2D<U>,
    coordinate_space: PhantomData<U>,
}

impl<U: CoordinateUnit> Rect2D<U> {
    pub fn new(top_left: Point2D<U>, bottom_right: Point2D<U>) -> Self {
        Self {
            top_left,
            bottom_right,
            coordinate_space: PhantomData,
        }
    }

    pub fn centered_on<U2>(center: Point2D<U2>, size: Size2D<U2>) -> Self
    where
        U2: CoordinateUnit,
        U: From<U2>,
    {
        let half_size = size / 2.0;
        Self {
            top_left: (center - Translate2D::from(half_size)).cast(),
            bottom_right: (center + Translate2D::from(half_size)).cast(),
            coordinate_space: PhantomData,
        }
    }

    pub fn from_size(size: Size2D<U>) -> Self {
        Self::centered_on(Point2D::new(0.0, 0.0), size)
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn center(&self) -> Point2D<U> {
        self.top_left + (self.bottom_right - self.top_left) / 2.0
    }

    pub fn contains<U2>(&self, point: Point2D<U2>) -> bool
    where
        U2: CoordinateUnit,
        U: From<U2>,
    {
        let point: Point2D<U> = point.cast();
        todo!()
    }

    pub fn translate<U2>(&mut self, vector: Translate2D<U2>)
    where
        U2: CoordinateUnit,
        U: From<U2>,
    {
        let vector: Translate2D<U> = vector.cast();
        todo!()
    }

    pub fn intersection<U2>(&self, other: &Rect2D<U2>) -> Option<Self>
    where
        U2: CoordinateUnit,
        U: From<U2>,
    {
        todo!()
    }

    pub fn move_into<U2>(&mut self, other: &Rect2D<U2>) -> Option<Self>
    where
        U2: CoordinateUnit,
        U: From<U2>,
    {
        todo!()
    }

    pub fn cast<Dst: CoordinateUnit>(self) -> Rect2D<Dst>
    where
        Dst: From<U>,
    {
        Rect2D::new(self.top_left.cast(), self.bottom_right.cast())
    }
}

impl<U: CoordinateUnit> Clone for Rect2D<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U: CoordinateUnit> Copy for Rect2D<U> {}
