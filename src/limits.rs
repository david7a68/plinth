//! Static limits and constraints.

/// The maximum number of UTF-8 bytes that can be used to represent a path to a
/// resource.
pub const RES_PATH_LENGTH_MAX: usize = 1024;

pub use crate::graphics::limits::*;
pub use crate::system::limits::*;
use crate::{core::limit::Limit, HashedStr};

pub struct ResourcePath<'a> {
    pub path: HashedStr<'a>,
}

impl<'a> ResourcePath<'a> {
    pub fn new(path: impl Into<HashedStr<'a>>) -> Option<Self> {
        let path = path.into();
        (path.string.len() < RES_PATH_LENGTH_MAX).then_some(Self { path })
    }
}

impl Limit<usize> for ResourcePath<'_> {
    const ASSERT_MESSAGE: &'static str = "Resource path length out of limits";

    fn min() -> usize {
        0
    }

    fn max() -> usize {
        RES_PATH_LENGTH_MAX
    }

    fn clamp(&mut self) {
        unimplemented!()
    }

    fn limit_check(&self) -> bool {
        self.path.len() <= Self::max()
    }
}
