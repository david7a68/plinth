use std::hash::{BuildHasher, Hasher};

pub mod arena;
pub mod limit;
pub mod slotmap;
pub mod static_lru_cache;

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

pub type PassthroughHashMap<T> = std::collections::HashMap<u64, T, PassthroughBuildHasher>;
