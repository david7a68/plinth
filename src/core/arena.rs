//! Fixed-size arena allocator.

use std::{
    alloc::{alloc, Layout},
    cell::Cell,
    marker::PhantomData,
    mem::{align_of, MaybeUninit},
    ops::{
        Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive,
        RangeTo, RangeToInclusive,
    },
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("system out of memory, could not allocate arena")]
    SysOutOfMemory,
    #[error("insufficient capacity in the arena, allocation failed")]
    InsufficientCapacity,
}

pub struct Arena {
    root: *mut u8,
    stop: *mut u8,
    next: Cell<*mut u8>,
}

impl Arena {
    pub fn new(size: usize) -> Result<Self, Error> {
        assert!(isize::try_from(size).is_ok(), "size exceeds isize::MAX");

        let root = {
            let layout = Layout::from_size_align(size, 8).unwrap();
            unsafe { alloc(layout) }
        };

        if root.is_null() {
            Err(Error::SysOutOfMemory)
        } else {
            // SAFETY: `root` is an aligned, non-null pointer to `size` bytes of
            // memory and ownership is transferred to the `Arena` instance.
            let slice = unsafe { from_raw_parts_mut(root, size) };

            Ok(Self::with_memory(slice))
        }
    }
}

impl Arena {
    #[must_use]
    pub fn with_memory(memory: &'static mut [u8]) -> Self {
        let root = memory.as_mut_ptr();

        // SAFETY: `root` is an aligned, non-null pointer to `memory.len()`
        // bytes. Adding `memory.len()` puts the pointer at one byte past the
        // end of the slice.
        let stop = unsafe { root.add(memory.len()) };

        Self {
            root,
            stop,
            next: Cell::new(root),
        }
    }
}

impl Arena {
    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, Error> {
        let align = self.next.get().align_offset(layout.align());

        // SAFETY: `next` and `stop` belong to the same allocation.
        let capacity = unsafe { self.stop.offset_from(self.next.get()) };

        let required = align + layout.size();

        debug_assert!(isize::try_from(required).is_ok());
        if required as isize > capacity {
            eprintln!("Attempted to allocate {required} bytes but only had {capacity} bytes free");
            return Err(Error::InsufficientCapacity);
        }

        // SAFETY: `next` has been checked to have enough capacity for the
        // requested allocation.
        let ptr = unsafe {
            let ptr = self.next.get().add(align);
            self.next.set(ptr.add(layout.size()));
            NonNull::new_unchecked(ptr)
        };

        Ok(ptr)
    }

    pub fn alloc_slice<T>(&self, len: usize) -> Result<&mut [MaybeUninit<T>], Error> {
        let mem = self.alloc(Layout::array::<T>(len).unwrap())?;
        debug_assert_eq!(mem.as_ptr().align_offset(align_of::<T>()), 0, "alignment");

        let ptr = mem.cast::<MaybeUninit<T>>();
        let arr = unsafe { from_raw_parts_mut(ptr.as_ptr(), len) };

        Ok(arr)
    }

    pub fn make<T>(&self, value: T) -> Result<Box<T>, Error> {
        self.make_with(|| value)
    }

    pub fn make_with<T>(&self, f: impl FnOnce() -> T) -> Result<Box<T>, Error> {
        let ptr = self.alloc(Layout::new::<T>())?;
        debug_assert_eq!(ptr.as_ptr().align_offset(align_of::<T>()), 0, "alignment");

        let ptr = ptr.cast::<T>();

        unsafe { ptr.as_ptr().write(f()) };

        Ok(Box {
            ptr,
            phantom: PhantomData,
        })
    }

    pub fn make_array<T>(&self, cap: u32) -> Result<Array32<'_, T>, Error> {
        assert!(u32::try_from(cap).is_ok(), "len exceeds u32::MAX");

        let ptr = self.alloc(Layout::array::<T>(cap as usize).unwrap())?;
        debug_assert_eq!(ptr.as_ptr().align_offset(align_of::<T>()), 0, "alignment");

        let ptr = ptr.cast::<T>();

        Ok(Array32 {
            ptr,
            len: 0,
            cap,
            phantom: PhantomData,
        })
    }

    pub fn make_array_with<T>(&self, len: u32, f: impl Fn(u32) -> T) -> Result<Array32<T>, Error> {
        assert!(u32::try_from(len).is_ok(), "len exceeds u32::MAX");

        let mut array = self.make_array::<T>(len)?;

        for i in 0..len {
            unsafe { array.ptr.as_ptr().add(i as usize).write(f(i)) }
        }

        array.len = len;

        Ok(array)
    }

    pub fn make_array_from<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<&mut [T], Error> {
        let save = self.next.get();

        let align = self.next.get().align_offset(align_of::<T>());

        self.next.set(unsafe { self.next.get().add(align) });
        let start = self.next.get();

        let size = Layout::new::<T>().pad_to_align().size();

        let mut len = 0;
        let mut err = false;

        for value in iter {
            let capacity = unsafe { self.next.get().offset_from(self.stop) };

            if capacity < size as isize {
                unsafe { self.next.get().cast::<T>().write(value) };
                self.next.set(unsafe { self.next.get().add(size) });
                len += 1;
            } else {
                err = true;
                break;
            }
        }

        if err {
            let mut next = unsafe { save.add(align) };

            while next < self.next.get() {
                unsafe { next.cast::<T>().drop_in_place() };
                next = unsafe { next.add(size) };
            }

            self.next.set(save);

            Err(Error::InsufficientCapacity)
        } else {
            let slice = unsafe { from_raw_parts_mut(start.cast(), len) };
            Ok(slice)
        }
    }

    /// Shrinks the array to the new capacity, dropping any excess elements.
    ///
    /// This function returns true if the array was the most recent allocation
    /// in the arena and so was able to shrink in place.
    pub fn shrink_array<T>(&self, new_cap: u32, array: &mut Array32<T>) -> bool {
        if new_cap < array.len {
            for i in new_cap..array.len {
                unsafe { array.ptr.as_ptr().add(i as usize).drop_in_place() }
            }

            array.len = new_cap;
        }

        array.cap = new_cap;

        let is_last_alloc =
            unsafe { array.ptr.as_ptr().add(array.len as usize) }.cast() == self.next.get();

        if is_last_alloc {
            self.next
                .set(unsafe { array.ptr.as_ptr().add(new_cap as usize) }.cast());
            true
        } else {
            true
        }
    }

    /// Grows the array to the new capacity.
    ///
    /// This function returns true if the array was the most recent allocation
    /// in the arena and so was able to grow in place.
    ///
    /// # Errors
    ///
    /// This function will produce an error if the arena is unable to allocate
    /// the required memory.
    ///
    /// # Panics
    ///
    /// This function will panic if the new capacity exceeds `u32::MAX` items,
    /// or if the size of the array would exceed `isize::MAX` bytes.
    pub fn grow_array<T>(&self, new_cap: u32, array: &mut Array32<T>) -> Result<bool, Error> {
        // todo: under-grow if insufficient capacity remaining but more than
        // array.cap, then return new capacity. allocate whatever's left in the
        // arena.

        let is_last_alloc =
            unsafe { array.ptr.as_ptr().add(array.len as usize) }.cast() == self.next.get();

        if is_last_alloc {
            let extra = Layout::array::<T>((new_cap - array.cap) as usize).unwrap();

            let save = self.next.get();
            let extra = self.alloc(extra)?;
            debug_assert_eq!(extra.as_ptr(), save, "alloc more than expected");

            array.cap = new_cap as u32;

            Ok(true)
        } else {
            let ptr = self
                .alloc(Layout::array::<T>(new_cap as usize).unwrap())?
                .cast::<T>();

            let ptr_ = ptr.as_ptr();
            debug_assert_eq!(ptr_.align_offset(align_of::<T>()), 0, "alignment");
            unsafe { ptr_.copy_from_nonoverlapping(array.ptr.as_ptr(), array.len() as usize) };

            array.ptr = ptr;
            array.cap = new_cap as u32;

            // This just leaves the old array allocated but without any external
            // references. It's the best we can do without a `reset()`.

            Ok(false)
        }
    }

    /// Resets the arena allocator.
    ///
    /// Any allocations made from the arena will be lost.
    pub fn reset(&mut self) {
        self.next.set(self.root);
    }
}

pub struct Box<'arena, T: 'arena> {
    ptr: NonNull<T>,
    phantom: PhantomData<&'arena mut T>,
}

impl<'arena, T> Box<'arena, T> {
    pub fn new(arena: &'arena mut Arena, value: T) -> Result<Self, Error> {
        arena.make(value)
    }

    pub fn new_with(arena: &'arena mut Arena, f: impl FnOnce() -> T) -> Result<Self, Error> {
        arena.make_with(f)
    }

    #[must_use]
    pub fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    #[must_use]
    pub fn as_ptr_mut(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    #[must_use]
    pub fn into_inner(self) -> T {
        unsafe { std::ptr::read(self.ptr.as_ptr()) }

        // no need to drop, since we return the value - dz
    }
}

impl<T> Deref for Box<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for Box<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for Box<'_, T> {
    fn drop(&mut self) {
        unsafe { std::ptr::drop_in_place(self.ptr.as_ptr()) }
    }
}

pub struct Array32<'arena, T: 'arena> {
    ptr: NonNull<T>,
    len: u32,
    cap: u32,
    phantom: PhantomData<&'arena mut T>,
}

impl<'arena, T> Array32<'arena, T> {
    pub fn new(mem: &'arena Arena) -> Result<Self, Error> {
        mem.make_array(0)
    }

    pub fn with_capacity(mem: &'arena Arena, cap: u32) -> Result<Self, Error> {
        mem.make_array(cap)
    }

    #[must_use]
    pub fn cap(&self) -> u32 {
        self.cap as u32
    }

    #[must_use]
    pub fn len(&self) -> u32 {
        self.len as u32
    }

    #[must_use]
    pub fn get(&self, index: u32) -> Option<&T> {
        if index < self.len() {
            unsafe { self.ptr.as_ptr().add(index as usize).as_ref() }
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        if index < self.len() {
            unsafe { self.ptr.as_ptr().add(index as usize).as_mut() }
        } else {
            None
        }
    }

    #[must_use]
    pub fn slice(&self) -> &[T] {
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.len as usize) }
    }

    #[must_use]
    pub fn slice_mut(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.len as usize) }
    }

    #[must_use]
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(unsafe { self.ptr.as_ptr().add(self.len as usize).read() })
        } else {
            None
        }
    }

    pub fn push(&mut self, arena: &'arena Arena, value: T) -> Result<u32, Error> {
        if self.len == self.cap {
            arena.grow_array(self.cap as u32 * 2, self)?;
        }

        debug_assert!(self.len < self.cap, "grow failed");

        unsafe { self.ptr.as_ptr().add(self.len as usize).write(value) };
        self.len += 1;

        Ok(self.len as u32)
    }

    pub fn extend(
        &mut self,
        arena: &'arena Arena,
        it: impl IntoIterator<Item = T>,
    ) -> Result<u32, Error> {
        let mut it = it.into_iter();
        let len = it.size_hint().1.unwrap_or(it.size_hint().0);

        // optimize for known size
        if len > 0 {
            if let Some(len) = self.len.checked_add(len as u32) {
                if len > self.cap {
                    arena.grow_array(len as u32, self)?;
                }
            } else {
                return Err(Error::InsufficientCapacity);
            }

            for i in 0..len {
                if let Some(item) = it.next() {
                    unsafe { self.ptr.as_ptr().add(i).write(item) };
                } else {
                    break;
                }
            }
        }

        // slow path for unknown size
        for item in it {
            self.push(arena, item)?;
        }

        Ok(self.len())
    }

    #[must_use]
    pub fn try_push(&mut self, value: T) -> bool {
        if self.len < self.cap {
            unsafe { self.ptr.as_ptr().add(self.len as usize).write(value) };
            self.len += 1;
            true
        } else {
            false
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn trim(&mut self, arena: &'arena mut Arena) -> bool {
        arena.shrink_array(self.len(), self)
    }
}

impl<T> Deref for Array32<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.len() as usize) }
    }
}

impl<T> DerefMut for Array32<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.len() as usize) }
    }
}

impl<T> Index<u32> for Array32<'_, T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<T> Index<Range<u32>> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: Range<u32>) -> &Self::Output {
        &self.slice()[Range {
            start: range.start as usize,
            end: range.end as usize,
        }]
    }
}

impl<T> Index<RangeFrom<u32>> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: RangeFrom<u32>) -> &Self::Output {
        &self.slice()[RangeFrom {
            start: range.start as usize,
        }]
    }
}

impl<T> Index<RangeTo<u32>> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: RangeTo<u32>) -> &Self::Output {
        &self.slice()[RangeTo {
            end: range.end as usize,
        }]
    }
}

impl<T> Index<RangeToInclusive<u32>> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: RangeToInclusive<u32>) -> &Self::Output {
        &self.slice()[RangeToInclusive {
            end: range.end as usize,
        }]
    }
}

impl<T> Index<RangeInclusive<u32>> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: RangeInclusive<u32>) -> &Self::Output {
        &self.slice()[RangeInclusive::new(*range.start() as usize, *range.end() as usize)]
    }
}

impl<T> Index<RangeFull> for Array32<'_, T> {
    type Output = [T];

    fn index(&self, range: RangeFull) -> &Self::Output {
        self.slice()
    }
}

impl<T> IndexMut<u32> for Array32<'_, T> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl<T> IndexMut<Range<u32>> for Array32<'_, T> {
    fn index_mut(&mut self, range: Range<u32>) -> &mut Self::Output {
        &mut self.slice_mut()[Range {
            start: range.start as usize,
            end: range.end as usize,
        }]
    }
}

impl<T> IndexMut<RangeFrom<u32>> for Array32<'_, T> {
    fn index_mut(&mut self, range: RangeFrom<u32>) -> &mut Self::Output {
        &mut self.slice_mut()[RangeFrom {
            start: range.start as usize,
        }]
    }
}

impl<T> IndexMut<RangeTo<u32>> for Array32<'_, T> {
    fn index_mut(&mut self, range: RangeTo<u32>) -> &mut Self::Output {
        &mut self.slice_mut()[RangeTo {
            end: range.end as usize,
        }]
    }
}

impl<T> IndexMut<RangeToInclusive<u32>> for Array32<'_, T> {
    fn index_mut(&mut self, range: RangeToInclusive<u32>) -> &mut Self::Output {
        &mut self.slice_mut()[RangeToInclusive {
            end: range.end as usize,
        }]
    }
}

impl<T> IndexMut<RangeInclusive<u32>> for Array32<'_, T> {
    fn index_mut(&mut self, range: RangeInclusive<u32>) -> &mut Self::Output {
        &mut self.slice_mut()[RangeInclusive::new(*range.start() as usize, *range.end() as usize)]
    }
}

impl<T> IndexMut<RangeFull> for Array32<'_, T> {
    fn index_mut(&mut self, range: RangeFull) -> &mut Self::Output {
        &mut self.slice_mut()[range]
    }
}

impl<T> Drop for Array32<'_, T> {
    fn drop(&mut self) {
        for i in 0..self.len() as usize {
            unsafe { self.ptr.as_ptr().add(i).drop_in_place() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_heap() {
        let mut arena = Arena::new(1024).unwrap();

        assert_eq!(unsafe { arena.root.offset_from(arena.stop) }, -1024);
        assert_eq!(arena.root, arena.next.get());

        arena.reset(); //  no-op here
        assert_eq!(arena.root, arena.next.get());

        test_arena(arena);
    }

    #[test]
    fn arena_stack() {
        let mut arena =
            Arena::with_memory(std::boxed::Box::leak(std::boxed::Box::new([0u8; 1024])));

        assert_eq!(unsafe { arena.root.offset_from(arena.stop) }, -1024);
        assert_eq!(arena.root, arena.next.get());

        arena.reset(); //  no-op here
        assert_eq!(arena.root, arena.next.get());

        test_arena(arena);
    }

    fn test_arena(mut arena: Arena) {
        let a = arena.alloc(Layout::array::<u8>(1024).unwrap()).unwrap();
        assert_eq!(a.as_ptr(), arena.root);

        let b = arena.alloc(Layout::new::<usize>());
        b.unwrap_err();

        arena.reset();

        let c = arena.alloc(Layout::array::<u8>(3).unwrap()).unwrap();
        assert_eq!(c.as_ptr(), arena.root);

        for i in 0..10 {
            // test aligning to layout
            let p = arena.alloc(Layout::new::<usize>()).unwrap();
            assert_eq!(p.as_ptr(), unsafe {
                arena.root.add((i + 1) * std::mem::size_of::<usize>())
            });
        }

        // fill the rest of the arena
        let d = arena.alloc_slice::<usize>(117).unwrap();
        assert_eq!(d.len(), 117);

        let e = arena.alloc(Layout::new::<u8>());
        e.unwrap_err();
    }

    #[test]
    fn arena_box() {
        let mut dropped = false;

        struct T<'a> {
            dropped: &'a mut bool,
        }

        impl Drop for T<'_> {
            fn drop(&mut self) {
                *self.dropped = true;
            }
        }

        let arena = Arena::new(1024).unwrap();

        let t = arena
            .make(T {
                dropped: &mut dropped,
            })
            .unwrap();

        std::mem::drop(t);

        assert!(dropped);
    }

    #[test]
    fn arena_array() {
        // make_array

        // make_array_with

        // make_array_from

        // push

        // try_push

        // pop

        // trim

        // drop

        // reset

        // todo
    }
}
