use std::{marker::PhantomData, mem::MaybeUninit};

pub trait Key: Clone + Copy + PartialEq + Sized {
    #[must_use]
    fn new(index: u32, epoch: u32) -> Self;

    #[must_use]
    fn index(&self) -> u32;

    #[must_use]
    fn epoch(&self) -> u32;
}

macro_rules! new_key_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name {
            index: u32,
            epoch: u32,
        }

        impl $name {
            #[allow(dead_code)]
            #[must_use]
            pub fn new(index: u32, epoch: u32) -> Self {
                Self { index, epoch }
            }

            #[allow(dead_code)]
            #[must_use]
            pub fn index(&self) -> u32 {
                self.index
            }

            #[allow(dead_code)]
            #[must_use]
            pub fn epoch(&self) -> u32 {
                self.epoch
            }
        }

        impl crate::core::static_slot_map::Key for $name {
            fn new(index: u32, epoch: u32) -> Self {
                Self { index, epoch }
            }

            fn index(&self) -> u32 {
                self.index
            }

            fn epoch(&self) -> u32 {
                self.epoch
            }
        }
    };
}

pub(crate) use new_key_type;

new_key_type!(DefaultKey);

pub struct SlotMap<const CAPACITY: usize, V, K: Key = DefaultKey> {
    slots: [Slot<V>; CAPACITY],
    next_free: u32,
    num_free: u32,
    num_used: u32,
    _phantom: PhantomData<K>,
}

impl<const CAPACITY: usize, V, K: Key> SlotMap<CAPACITY, V, K> {
    pub fn new() -> Self {
        assert!(CAPACITY > 0, "capacity must be greater than 0");
        assert!(
            u32::try_from(CAPACITY).is_ok(),
            "capacity must be less than u32::MAX"
        );

        let mut slots = [(); CAPACITY].map(|()| Slot {
            next: 0,
            epoch: 0,
            value: MaybeUninit::uninit(),
        });

        let next_free = 0;

        #[allow(clippy::cast_possible_truncation)]
        for (i, slot) in slots[0..].iter_mut().enumerate() {
            slot.next = (i + 1) as u32;
        }

        slots.last_mut().unwrap().next = u32::MAX;

        Self {
            slots,
            next_free,
            num_free: CAPACITY as u32,
            num_used: 0,
            _phantom: PhantomData,
        }
    }

    pub fn has_key(&self, key: K) -> bool {
        let Some(slot) = self.slots.get(key.index() as usize) else {
            return false;
        };

        slot.epoch == key.epoch()
    }

    pub fn has_capacity(&self, capacity: usize) -> bool {
        usize::try_from(self.num_free).unwrap() >= capacity
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let slot = self.slots.get(key.index() as usize)?;

        if slot.epoch == key.epoch() {
            Some(unsafe { slot.value.assume_init_ref() })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let slot = self.slots.get_mut(key.index() as usize)?;

        if slot.epoch == key.epoch() {
            Some(unsafe { slot.value.assume_init_mut() })
        } else {
            None
        }
    }

    pub fn insert(&mut self, value: V) -> Result<K, V> {
        self.create(|_| value)
    }

    pub fn create(&mut self, f: impl FnOnce(K) -> V) -> Result<K, V> {
        if self.next_free == u32::MAX {
            return Err(f(Key::new(0, 0)));
        }

        let index = self.next_free as usize;
        let slot = &mut self.slots[index];

        self.next_free = slot.next;
        slot.value = MaybeUninit::new(f(Key::new(u32::try_from(index).unwrap(), slot.epoch)));

        self.num_free = self.num_free.checked_sub(1).unwrap();
        self.num_used = self.num_used.checked_add(1).unwrap();

        Ok(Key::new(u32::try_from(index).unwrap(), slot.epoch))
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let slot = self.slots.get_mut(key.index() as usize)?;

        if slot.epoch != key.epoch() {
            return None;
        }

        if let Some(epoch) = slot.epoch.checked_add(1) {
            slot.epoch = epoch;
            slot.next = self.next_free;
            self.next_free = key.index();
            self.num_free.checked_add(1).unwrap();
        } else {
            // no-op: epoch saturation, retire the slot
        }

        self.num_used = self.num_used.checked_sub(1).unwrap();

        Some(unsafe { slot.value.assume_init_read() })
    }
}

struct Slot<V> {
    next: u32,
    epoch: u32,
    value: MaybeUninit<V>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick() {
        let mut map = SlotMap::<2, u32>::new();
        let key = map.insert(1).unwrap();
        assert_eq!(*map.get(key).unwrap(), 1);

        let key = map.insert(255).unwrap();
        assert_eq!(*map.get(key).unwrap(), 255);

        *map.get_mut(key).unwrap() = u32::MAX;
        assert_eq!(*map.get(key).unwrap(), u32::MAX);

        assert!(map.insert(0).is_err());

        assert_eq!(map.remove(key), Some(u32::MAX));
        assert_eq!(map.remove(key), None);
    }
}
