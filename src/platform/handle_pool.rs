use std::{marker::PhantomData, mem::MaybeUninit};

pub struct Handle<T> {
    index: u16,
    generation: u16,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Handle<T> {
    pub unsafe fn retype<U>(self) -> Handle<U> {
        Handle {
            index: self.index,
            generation: self.generation,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Handle<{}>", std::any::type_name::<T>()))
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

pub struct HandlePool<T, const CAPACITY: usize, H = T> {
    slots: [Slot<T>; CAPACITY],
    free_head: Option<u16>,
    high_water_mark: usize,
    _handle_type: std::marker::PhantomData<H>,
}

impl<T, H, const CAPACITY: usize> HandlePool<T, CAPACITY, H> {
    const CAPACITY_OK: bool = {
        assert!(
            CAPACITY < u16::MAX as usize,
            "HandlePool capacity must be < u16::MAX"
        );
        true
    };

    const SENTINEL: u16 = u16::MAX;

    pub fn new() -> Self {
        let slots = {
            let mut slots = std::array::from_fn(|i| Slot {
                is_free: true,
                generation: 0,
                index_or_next: (i + 1) as u16,
                value: MaybeUninit::uninit(),
            });

            // todo: unwrap should be optimized out (need to check)
            slots.last_mut().unwrap().index_or_next = Self::SENTINEL;
            slots
        };

        Self {
            slots,
            free_head: Some(0),
            high_water_mark: 0,
            _handle_type: PhantomData,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<H>, &T)> {
        self.slots[..self.high_water_mark]
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                if slot.is_free {
                    None
                } else {
                    let handle = Handle {
                        index: i as u16,
                        generation: slot.generation,
                        _marker: std::marker::PhantomData,
                    };

                    Some((handle, unsafe { slot.value.assume_init_ref() }))
                }
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<H>, &mut T)> {
        self.slots[..self.high_water_mark]
            .iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| {
                if slot.is_free {
                    None
                } else {
                    let handle = Handle {
                        index: i as u16,
                        generation: slot.generation,
                        _marker: std::marker::PhantomData,
                    };

                    Some((handle, unsafe { slot.value.assume_init_mut() }))
                }
            })
    }

    pub fn get(&self, handle: Handle<H>) -> Option<&T> {
        let slot = Self::check(self.slots.get(handle.index as usize)?, handle)?;
        Some(unsafe { slot.value.assume_init_ref() })
    }

    pub fn get_mut(&mut self, handle: Handle<H>) -> Option<&mut T> {
        let slot = Self::check_mut(self.slots.get_mut(handle.index as usize)?, handle)?;
        Some(unsafe { slot.value.assume_init_mut() })
    }

    pub fn insert(&mut self, value: T) -> Option<(Handle<H>, &mut T)> {
        let index = self.free_head?;
        let slot = &mut self.slots[index as usize];

        debug_assert!(slot.is_free);

        slot.is_free = false;
        slot.value = MaybeUninit::new(value);

        self.free_head = if slot.index_or_next == Self::SENTINEL {
            None
        } else {
            Some(slot.index_or_next)
        };

        slot.index_or_next = Self::SENTINEL;
        self.high_water_mark = self.high_water_mark.max(index as usize + 1);

        Some((
            Handle {
                index,
                generation: slot.generation,
                _marker: std::marker::PhantomData,
            },
            unsafe { slot.value.assume_init_mut() },
        ))
    }

    pub fn remove(&mut self, handle: Handle<H>) -> Option<T> {
        let slot = Self::check_mut(self.slots.get_mut(handle.index as usize)?, handle)?;
        let value = unsafe { slot.value.assume_init_read() };

        slot.is_free = true;
        slot.value = MaybeUninit::uninit();

        // only reuse the slot if we don't have to reuse cookie generations
        if slot.generation < Self::SENTINEL {
            slot.index_or_next = self.free_head.unwrap_or(Self::SENTINEL);
            self.free_head = Some(handle.index);
            slot.generation += 1;
        }

        Some(value)
    }

    fn check(slot: &Slot<T>, handle: Handle<H>) -> Option<&Slot<T>> {
        (!slot.is_free && slot.generation == handle.generation).then_some(slot)
    }

    fn check_mut(slot: &mut Slot<T>, handle: Handle<H>) -> Option<&mut Slot<T>> {
        (!slot.is_free && slot.generation == handle.generation).then_some(slot)
    }
}

struct Slot<T> {
    is_free: bool,
    generation: u16,
    index_or_next: u16,
    value: MaybeUninit<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let mut pool = HandlePool::<u32, 4>::new();

        let (h0, _) = pool.insert(0).unwrap();
        let (h1, _) = pool.insert(1).unwrap();
        let (h2, _) = pool.insert(2).unwrap();
        let (h3, _) = pool.insert(3).unwrap();

        assert_eq!(pool.get(h0), Some(&0));
        assert_eq!(pool.get(h1), Some(&1));
        assert_eq!(pool.get(h2), Some(&2));
        assert_eq!(pool.get(h3), Some(&3));

        assert_eq!(pool.get_mut(h0), Some(&mut 0));
        assert_eq!(pool.get_mut(h1), Some(&mut 1));
        assert_eq!(pool.get_mut(h2), Some(&mut 2));
        assert_eq!(pool.get_mut(h3), Some(&mut 3));
    }

    #[test]
    fn iter() {
        let mut pool = HandlePool::<u32, 4>::new();

        let (h0, _) = pool.insert(0).unwrap();
        let (h1, _) = pool.insert(1).unwrap();
        let (h2, _) = pool.insert(2).unwrap();
        let (h3, _) = pool.insert(3).unwrap();

        println!("{:?}", pool.iter().collect::<Vec<_>>());

        assert!(pool.iter().map(|(h, _)| h).eq([h0, h1, h2, h3]));
    }
}
