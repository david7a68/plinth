use std::ops::Deref;

use crate::core::limit::Limit;

pub const SYS_WINDOW_COUNT_MAX: usize = 8;
pub const SYS_WINDOW_COORD_MAX: i16 = 8192;
pub const SYS_WINDOW_COORD_MIN: i16 = 100;
pub const SYS_WINDOW_TITLE_LENGTH_MAX: usize = 255;

pub struct WindowTitle<'a> {
    pub title: &'a str,
}

impl<'a> WindowTitle<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { title }.clamped()
    }
}

impl Limit<usize> for WindowTitle<'_> {
    const ASSERT_MESSAGE: &'static str = "Window title too long";

    #[inline]
    fn min() -> usize {
        0
    }

    #[inline]
    fn max() -> usize {
        SYS_WINDOW_TITLE_LENGTH_MAX
    }

    #[inline]
    fn clamp(&mut self) {
        self.title = &self.title[..self.title.len().min(Self::max())];
    }

    #[inline]
    fn limit_check(&self) -> bool {
        self.title.len() <= SYS_WINDOW_TITLE_LENGTH_MAX
    }
}

impl Deref for WindowTitle<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.title
    }
}
