use std::hash::{BuildHasher, Hasher};

pub mod arena;
pub mod limits;
pub mod static_slot_map;

pub struct PassthroughBuildHasher {}

impl PassthroughBuildHasher {
    pub fn new() -> Self {
        PassthroughBuildHasher {}
    }
}

impl BuildHasher for PassthroughBuildHasher {
    type Hasher = PassthroughHasher;

    fn build_hasher(&self) -> Self::Hasher {
        PassthroughHasher { val: 0 }
    }
}

pub struct PassthroughHasher {
    val: u64,
}

impl Hasher for PassthroughHasher {
    fn finish(&self) -> u64 {
        self.val
    }

    fn write(&mut self, _bytes: &[u8]) {
        // no-op
    }

    fn write_u64(&mut self, i: u64) {
        self.val = i;
    }
}
