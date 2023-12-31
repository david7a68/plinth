pub struct BitMap32(i32);

impl BitMap32 {
    const MAX_BITS: u32 = 32;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_set(&self, index: u32) -> bool {
        self.0 & (1 << index) != 0
    }

    pub fn next_unset(&mut self) -> Option<u32> {
        let inverted = !self.0;
        if inverted != 0 {
            let index = inverted.trailing_zeros();
            self.0 |= 1 << index;
            Some(index)
        } else {
            None
        }
    }

    pub fn set(&mut self, index: u32, value: bool) {
        if value {
            self.0 |= 1 << index;
        } else {
            self.0 &= !(1 << index);
        }
    }
}

/// A wrapper type around a RwLock that enforces read-only behavior.
pub struct AcRead<'a, T> {
    inner: &'a parking_lot::RwLock<T>,
}

impl<'a, T> AcRead<'a, T> {
    pub fn new(inner: &'a parking_lot::RwLock<T>) -> Self {
        Self { inner }
    }

    pub fn read(&self) -> ReadGuard<'a, T> {
        ReadGuard {
            inner: self.inner.read(),
        }
    }
}

pub struct ReadGuard<'a, T> {
    inner: parking_lot::RwLockReadGuard<'a, T>,
}

impl<'a, T> std::ops::Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[repr(align(64))]
pub struct Pad64<T>(pub T);

impl<T> std::ops::Deref for Pad64<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Pad64<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap32() {
        let mut occupancy = BitMap32::new();

        for i in 0..BitMap32::MAX_BITS {
            assert_eq!(occupancy.next_unset(), Some(i as u32));
        }

        assert_eq!(occupancy.next_unset(), None);

        occupancy.set(0, false);
        assert!(!occupancy.is_set(0));
        assert_eq!(occupancy.next_unset(), Some(0));

        occupancy.set(12, false);
        assert!(!occupancy.is_set(12));
        occupancy.set(12, true);
        assert!(occupancy.is_set(12));
    }
}
