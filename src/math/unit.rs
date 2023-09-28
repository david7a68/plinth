pub trait CoordinateUnit {}

pub struct UndefinedUnit {}

impl CoordinateUnit for UndefinedUnit {}

pub struct Pixel;

impl CoordinateUnit for Pixel {}
