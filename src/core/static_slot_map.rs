use std::{marker::PhantomData, mem::MaybeUninit};

pub trait Key: Clone + Copy + PartialEq + Sized {
    fn new(index: u32, epoch: u32) -> Self;

    fn index(&self) -> u32;

    fn epoch(&self) -> u32;
}

macro_rules! new_key_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq)]
        pub struct $name {
            index: u32,
            epoch: u32,
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

new_key_type!(DefaultKey);

pub struct SlotMap<const CAPACITY: usize, V, K: Key = DefaultKey> {
    slots: [Slot<V>; CAPACITY],
    next_free: u32,
    _phantom: PhantomData<K>,
}

impl<const CAPACITY: usize, V, K: Key> SlotMap<CAPACITY, V, K> {
    pub fn new(default: V) -> Self {
        assert!(CAPACITY > 0, "capacity must be greater than 0");
        assert!(
            CAPACITY <= u32::MAX as usize,
            "capacity must be less than u32::MAX"
        );

        let mut slots = [(); CAPACITY].map(|_| Slot {
            next: 0,
            epoch: 0,
            value: MaybeUninit::uninit(),
        });

        let next_free = 1;

        for (i, slot) in slots[1..].iter_mut().enumerate() {
            slot.next = (i + 1) as u32;
        }

        slots[0].value.write(default);
        slots.last_mut().unwrap().next = 0;

        Self {
            slots,
            next_free,
            _phantom: PhantomData,
        }
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
        if self.next_free == 0 {
            return Err(value);
        }

        let index = self.next_free as usize;
        let slot = &mut self.slots[index];

        self.next_free = slot.next;
        slot.value.write(value);

        Ok(Key::new(index as u32, slot.epoch))
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let slot = self.slots.get_mut(key.index() as usize)?;

        if slot.epoch != key.epoch() {
            return None;
        }

        match slot.epoch.checked_add(1) {
            Some(epoch) => {
                slot.epoch = epoch;
                slot.next = self.next_free;
                self.next_free = key.index();
            }
            None => {
                // retire the slot, so don't add it to the free list
            }
        }

        Some(unsafe { slot.value.assume_init_read() })
    }
}

struct Slot<V> {
    next: u32,
    epoch: u32,
    value: MaybeUninit<V>,
}
