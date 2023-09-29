use std::time::Duration;

use super::Vec2;

pub struct Pixels {}

pub struct PixelsPerSecond {}

impl std::ops::Mul<Duration> for Vec2<PixelsPerSecond> {
    type Output = Vec2<Pixels>;

    fn mul(self, rhs: Duration) -> Self::Output {
        let seconds = rhs.as_secs_f64();
        Vec2::new(self.x * seconds, self.y * seconds)
    }
}
