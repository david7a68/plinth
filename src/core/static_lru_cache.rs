use std::collections::HashMap;

use crate::Hash;

use super::PassthroughBuildHasher;

pub struct LruCache<const SIZE: usize, T> {
    hash: HashMap<u64, SmallIndex, PassthroughBuildHasher>,
    list: Box<LruList<SIZE, (Hash, T)>>,
}

impl<const SIZE: usize, T> LruCache<SIZE, T> {
    pub fn new() -> Self {
        Self {
            hash: HashMap::with_capacity_and_hasher(SIZE, PassthroughBuildHasher::new()),
            list: Box::new(LruList::new()),
        }
    }

    pub fn get_or_insert_with(&mut self, key: Hash, f: impl Fn() -> T) -> (&T, Option<T>) {
        let (index, old) = if let Some(&index) = self.hash.get(&key.0) {
            (index, None)
        } else {
            let value = f();
            let (index, old) = self.list.insert((key, value));

            if let Some((old_key, _)) = old {
                self.hash.remove(&old_key.0);
            }

            self.hash.insert(key.0, index);
            (index, old.map(|(_, value)| value))
        };

        let value = &self.list.get_mut(index).unwrap().1;

        (value, old)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SmallIndex(u16);

/// A fixed-size linked list that discards the oldest item when inserting while
/// full.
struct LruList<const SIZE: usize, T> {
    item: [Option<T>; SIZE],
    prev: [SmallIndex; SIZE],
    next: [SmallIndex; SIZE],
    head: SmallIndex,
    size: SmallIndex,
}

impl<const SIZE: usize, T> LruList<SIZE, T> {
    fn new() -> Self {
        assert!(u16::try_from(SIZE).is_ok());

        Self {
            item: [(); SIZE].map(|_| None),
            prev: [SmallIndex(0); SIZE],
            next: [SmallIndex(0); SIZE],
            head: SmallIndex(0),
            size: SmallIndex(0),
        }
    }

    fn front(&self) -> Option<(SmallIndex, &T)> {
        if self.size.0 > 0 {
            let index = self.head.0 as usize;
            self.item[index].as_ref().map(|item| (self.head, item))
        } else {
            None
        }
    }

    fn front_mut(&mut self) -> Option<(SmallIndex, &mut T)> {
        if self.size.0 > 0 {
            let index = self.head.0 as usize;
            self.item[index].as_mut().map(|item| (self.head, item))
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: SmallIndex) -> Option<&mut T> {
        self.item[self.bring_to_front(index)].as_mut()
    }

    fn insert(&mut self, value: T) -> (SmallIndex, Option<T>) {
        if (self.size.0 as usize) < SIZE {
            let index = self.size;
            self.item[index.0 as usize] = Some(value);

            if self.size.0 == 0 {
                self.head = index;
                self.prev[index.0 as usize] = index;
                self.next[index.0 as usize] = index;
            } else {
                let last = self.prev[self.head.0 as usize];

                self.prev[index.0 as usize] = last;
                self.next[last.0 as usize] = index;

                self.next[index.0 as usize] = self.head;
                self.prev[self.head.0 as usize] = index;

                self.head = index;
            }

            self.size.0 += 1;

            (index, None)
        } else {
            let last = self.prev[self.head.0 as usize];

            let item = self.item[last.0 as usize].replace(value);

            // simply rotate the list by one
            self.head = last;

            (last, item)
        }
    }

    fn bring_to_front(&mut self, index: SmallIndex) -> usize {
        let index_ = index.0 as usize;

        // remove the item from the list
        self.next[self.prev[index_].0 as usize] = self.next[index_];
        self.prev[self.next[index_].0 as usize] = self.prev[index_];

        // adjust self
        self.next[index_] = self.head;
        self.prev[index_] = self.prev[self.head.0 as usize];

        // adjust prev
        self.next[self.prev[index_].0 as usize] = index;

        // adjust next
        self.prev[self.next[index_].0 as usize] = index;

        // update head
        self.head = index;

        self.head.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        let mut cache = LruCache::<2, u32>::new();
        // insert 1
        assert_eq!(cache.get_or_insert_with(Hash::of(&12), || 12), (&12, None));

        // repeat is a no-op
        assert_eq!(cache.get_or_insert_with(Hash::of(&12), || 12), (&12, None));

        // insert 2
        assert_eq!(cache.get_or_insert_with(Hash::of(&13), || 13), (&13, None));

        // repeat 1 is a no-op
        assert_eq!(cache.get_or_insert_with(Hash::of(&12), || 12), (&12, None));

        // repeat 2 is a no-op
        assert_eq!(cache.get_or_insert_with(Hash::of(&13), || 13), (&13, None));

        // insert 3, 1 is evicted
        assert_eq!(
            cache.get_or_insert_with(Hash::of(&14), || 14),
            (&14, Some(12))
        );
    }
}
