pub trait Key: Copy + std::fmt::Debug + Sized + Eq + Ord + std::hash::Hash {
    fn new(index: u32, epoch: u32) -> Self;

    fn index(&self) -> u32;

    fn epoch(&self) -> u32;

    fn to_raw(self) -> u32;

    fn from_raw(raw: u32) -> Self;

    fn index_bits() -> u32;

    fn epoch_max() -> u32;

    fn index_max() -> u32;
}

macro_rules! new_key_type {
    ($(#[$meta:meta])* $name:ident) => {
        $crate::core::slotmap::new_key_type!($(#[$meta])* $name, 16);
    };
    ($(#[$meta:meta])* $name:ident, $index_bits:expr) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            raw: u32,
        }

        impl $name {
            pub const INDEX_BITS: u32 = $index_bits;
            pub const INDEX_MAX: u32 = (1 << Self::INDEX_BITS) - 1;
            pub const EPOCH_MAX: u32 = !Self::INDEX_MAX >> Self::INDEX_BITS;

            const INDEX_MASK: u32 = Self::INDEX_MAX;

            #[must_use]
            #[allow(dead_code)]
            pub const fn new(index: u32, epoch: u32) -> Self {
                debug_assert!(Self::INDEX_BITS > 0);
                debug_assert!(index <= Self::INDEX_MAX);
                debug_assert!(epoch <= Self::EPOCH_MAX);
                debug_assert!(Self::INDEX_MAX ^ (Self::EPOCH_MAX << Self::INDEX_BITS) == u32::MAX);

                Self {
                    raw: index | (epoch << Self::INDEX_BITS),
                }
            }

            #[must_use]
            #[allow(dead_code)]
            pub const fn index(&self) -> u32 {
                self.raw & Self::INDEX_MASK
            }

            #[must_use]
            #[allow(dead_code)]
            pub const fn epoch(&self) -> u32 {
                self.raw >> Self::INDEX_BITS
            }

            #[must_use]
            #[allow(dead_code)]
            pub const fn to_raw(self) -> u32 {
                self.raw
            }

            #[must_use]
            #[allow(dead_code)]
            pub const fn from_raw(raw: u32) -> Self {
                Self { raw }
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("index", &self.index())
                    .field("epoch", &self.epoch())
                    .finish()
            }
        }

        impl $crate::core::slotmap::Key for $name {
            fn new(index: u32, epoch: u32) -> Self {
                Self::new(index, epoch)
            }

            fn index(&self) -> u32 {
                self.index()
            }

            fn epoch(&self) -> u32 {
                self.epoch()
            }

            fn to_raw(self) -> u32 {
                self.to_raw()
            }

            fn from_raw(raw: u32) -> Self {
                Self::from_raw(raw)
            }

            fn index_bits() -> u32 {
                Self::INDEX_BITS
            }

            fn index_max() -> u32 {
                Self::INDEX_MAX
            }

            fn epoch_max() -> u32 {
                Self::EPOCH_MAX
            }
        }
    };
}

use core::panic;
use std::mem::MaybeUninit;

pub(crate) use new_key_type;

new_key_type!(DefaultKey);

pub struct SlotMap<V, K: Key = DefaultKey> {
    slots: Vec<Slot<K, V>>,
    flags: Vec<Flags>,
    free_index: u32,
    free_count: u32,
}

impl<V, K: Key> SlotMap<V, K> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            flags: Vec::new(),
            free_index: 0,
            free_count: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            flags: Vec::with_capacity(capacity),
            free_index: 0,
            free_count: 0,
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let slot = self.slots.get(key.index() as usize)?;

        if slot.next_and_epoch == key {
            Some(unsafe { slot.value.assume_init_ref() })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let slot = self.slots.get_mut(key.index() as usize)?;

        if slot.next_and_epoch == key {
            Some(unsafe { slot.value.assume_init_mut() })
        } else {
            None
        }
    }

    pub fn create(&mut self, f: impl FnOnce(K) -> V) -> K {
        let (key, slot) = self.alloc_slot();
        slot.value = MaybeUninit::new(f(key));
        key
    }

    pub fn insert(&mut self, value: V) -> K {
        let (key, slot) = self.alloc_slot();
        slot.value = MaybeUninit::new(value);
        key
    }

    fn alloc_slot(&mut self) -> (K, &mut Slot<K, V>) {
        if self.free_count > 0 {
            let index = self.free_index;
            let slot = &mut self.slots[index as usize];

            self.free_index = slot.next_and_epoch.index();
            self.free_count -= 1;

            slot.next_and_epoch = K::new(index, slot.next_and_epoch.epoch());
            (slot.next_and_epoch, slot)
        } else if self.slots.len() < K::index_max() as usize {
            let index = self.slots.len() as u32;
            let slot = Slot {
                next_and_epoch: K::new(index, 0),
                value: MaybeUninit::uninit(),
            };

            self.slots.push(slot);
            self.flags.push(Flags { used: true });

            (K::new(index, 0), self.slots.last_mut().unwrap())
        } else {
            panic!("SlotMap is full");
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let slot = self.slots.get_mut(key.index() as usize)?;

        if slot.next_and_epoch == key {
            let value = unsafe { slot.value.assume_init_read() };
            self.flags[key.index() as usize].used = false;

            if slot.next_and_epoch.epoch() < K::epoch_max() {
                slot.next_and_epoch = K::new(self.free_index, key.epoch() + 1);
                self.free_index = key.index();
                self.free_count += 1;
            } else {
                // saturated, retire the slot
            }

            Some(value)
        } else {
            None
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(K, &V) -> bool) {
        for (i, (flags, slot)) in (self.flags.iter_mut().zip(self.slots.iter_mut())).enumerate() {
            if flags.used {
                debug_assert_eq!(slot.next_and_epoch.index() as usize, i);

                let keep = f(slot.next_and_epoch, unsafe { slot.value.assume_init_ref() });
                if !keep {
                    flags.used = false;

                    if slot.next_and_epoch.epoch() < K::epoch_max() {
                        slot.next_and_epoch =
                            K::new(self.free_index, slot.next_and_epoch.epoch() + 1);
                        self.free_index = i as u32;
                        self.free_count += 1;
                    }
                }
            }
        }
    }
}

struct Slot<K: Key, V> {
    next_and_epoch: K,
    value: MaybeUninit<V>,
}

struct Flags {
    used: bool,
}

#[cfg(test)]
mod tests {
    use super::{DefaultKey, Key, SlotMap};

    #[test]
    fn key16_type() {
        let key = DefaultKey::new(1, 1);
        assert_eq!(key.index(), 1);
        assert_eq!(key.epoch(), 1);

        let key = DefaultKey::new(u16::MAX as u32, u16::MAX as u32);
        assert_eq!(key.index(), u16::MAX as u32);
        assert_eq!(key.epoch(), u16::MAX as u32);

        let key = DefaultKey::new(u16::MAX as u32, 0);
        assert_eq!(key.index(), u16::MAX as u32);
        assert_eq!(key.epoch(), 0);

        let key = DefaultKey::new(0, u16::MAX as u32);
        assert_eq!(key.index(), 0);
    }

    #[test]
    #[should_panic]
    fn key16_index_out_of_range() {
        let _ = DefaultKey::new(u16::MAX as u32 + 1, 0);
    }

    #[test]
    #[should_panic]
    fn key16_epoch_out_of_range() {
        let _ = DefaultKey::new(0, u16::MAX as u32 + 1);
    }

    #[test]
    fn key8_type() {
        new_key_type!(Key8, 8);

        let key = Key8::new(1, 1);
        assert_eq!(key.index(), 1);
        assert_eq!(key.epoch(), 1);

        let key = Key8::new(u8::MAX as u32, (1 << 24) - 1);
        assert_eq!(key.index(), u8::MAX as u32);
        assert_eq!(key.epoch(), (1 << 24) - 1);
    }

    #[test]
    #[should_panic]
    fn key8_index_out_of_range() {
        new_key_type!(Key8, 8);

        let _ = Key8::new(u8::MAX as u32 + 1, 0);
    }

    #[test]
    #[should_panic]
    fn key8_epoch_out_of_range() {
        new_key_type!(Key8, 8);
        let _ = Key8::new(0, 1 << 24);
    }

    #[test]
    fn insert_1000() {
        let mut map = SlotMap::<u32>::new();

        for i in 0..1_000 {
            let key = map.insert(i);
            assert_eq!(*map.get(key).unwrap(), i);
        }

        assert_eq!(map.slots.len(), 1_000);
    }

    #[test]
    fn reuse_slots() {
        let mut map = SlotMap::<u32>::new();

        for i in (0..1000).rev() {
            let key = map.insert(i);
            assert_eq!(map.remove(key), Some(i));
        }

        assert_eq!(map.slots.len(), 1);
    }

    #[test]
    fn saturate_slot() {
        new_key_type!(Key22, 28);
        assert_eq!(Key22::EPOCH_MAX, 15);
        assert_eq!(Key22::epoch_max(), 15);
        assert_eq!(Key22::EPOCH_MAX, Key22::epoch_max());

        let mut map = SlotMap::<u32, Key22>::new();

        for i in 0..32 {
            let key = map.insert(i);
            assert_eq!(map.remove(key), Some(i));
        }

        assert_eq!(map.slots.len(), 2);
        assert_eq!(map.slots[0].next_and_epoch.epoch(), Key22::EPOCH_MAX);
        assert_eq!(map.free_count, 0);

        let key = map.insert(0);
        map.remove(key);

        assert_eq!(map.slots.len(), 3);
    }

    #[test]
    fn retain_even_odd() {
        let mut map = SlotMap::<u32>::new();

        for i in 0..100 {
            map.insert(i);
        }

        map.retain(|_, &v| v % 2 == 0);
        assert_eq!(map.free_count, 50);

        map.retain(|_, &v| v % 2 == 1);
        assert_eq!(map.free_count, 100);
    }
}
