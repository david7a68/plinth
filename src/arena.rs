//! Fixed-size arena allocator.

use std::{
    alloc::{alloc, Layout},
    cell::Cell,
    marker::PhantomData,
    mem::{align_of, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

pub enum Error {
    SysOutOfMemory,
    InsufficientCapacity,
}

pub struct Arena<'a> {
    root: *mut u8,
    stop: *mut u8,
    next: Cell<*mut u8>,
    phantom: PhantomData<&'a mut u8>,
}

impl Arena<'static> {
    #[must_use]
    pub fn new(size: usize) -> Result<Self, Error> {
        assert!(size <= isize::MAX as usize, "size exceeds isize::MAX");

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

impl<'a> Arena<'a> {
    #[must_use]
    pub fn with_memory(memory: &'a mut [u8]) -> Self {
        let root = memory.as_mut_ptr();

        // SAFETY: `root` is an aligned, non-null pointer to `memory.len()`
        // bytes. Adding `memory.len()` puts the pointer at one byte past the
        // end of the slice.
        let stop = unsafe { root.add(memory.len()) };

        Self {
            root,
            stop,
            next: Cell::new(root),
            phantom: PhantomData,
        }
    }
}

impl Arena<'_> {
    #[must_use]
    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, Error> {
        let align = self.next.get().align_offset(layout.align());

        // SAFETY: `next` and `stop` belong to the same allocation.
        let capacity = unsafe { self.next.get().offset_from(self.stop) };

        let required = align + layout.size();

        debug_assert!(capacity >= 0);
        if required > capacity as usize {
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

    pub fn make_array<T>(&self, cap: usize) -> Result<Array<T>, Error> {
        assert!(cap <= u32::MAX as usize, "len exceeds u32::MAX");

        let ptr = self.alloc(Layout::array::<T>(cap).unwrap())?;
        debug_assert_eq!(ptr.as_ptr().align_offset(align_of::<T>()), 0, "alignment");

        let ptr = ptr.cast::<T>();

        Ok(Array {
            ptr,
            len: 0,
            cap: cap as u32,
            phantom: PhantomData,
        })
    }

    pub fn make_array_with<T>(
        &self,
        len: usize,
        f: impl Fn(usize) -> T,
    ) -> Result<Array<T>, Error> {
        assert!(len <= u32::MAX as usize, "len exceeds u32::MAX");

        let mut array = self.make_array::<T>(len)?;

        for i in 0..len {
            unsafe { array.ptr.as_ptr().add(i).write(f(i)) }
        }

        array.len = len as u32;

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

        for value in iter.into_iter() {
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
    pub fn shrink_array<T>(&self, new_cap: usize, array: &mut Array<T>) -> bool {
        if new_cap < array.len as usize {
            for i in new_cap..array.len as usize {
                unsafe { array.ptr.as_ptr().add(i).drop_in_place() }
            }

            array.len = new_cap as u32;
        }

        array.cap = new_cap as u32;

        let is_last_alloc =
            unsafe { array.ptr.as_ptr().add(array.len as usize) }.cast() == self.next.get();

        if is_last_alloc {
            self.next
                .set(unsafe { array.ptr.as_ptr().add(new_cap) }.cast());
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
    pub fn grow_array<T>(&self, new_cap: usize, array: &mut Array<T>) -> Result<bool, Error> {
        assert!(new_cap <= u32::MAX as usize, "len exceeds u32::MAX");

        let is_last_alloc =
            unsafe { array.ptr.as_ptr().add(array.len as usize) }.cast() == self.next.get();

        if is_last_alloc {
            let extra = Layout::array::<T>(new_cap - array.cap as usize).unwrap();

            let save = self.next.get();
            let extra = self.alloc(extra)?;
            debug_assert_eq!(extra.as_ptr(), save, "alloc more than expected");

            array.cap = new_cap as u32;

            Ok(true)
        } else {
            let ptr = self
                .alloc(Layout::array::<T>(new_cap).unwrap())?
                .cast::<T>();
            debug_assert_eq!(ptr.as_ptr().align_offset(align_of::<T>()), 0, "alignment");

            unsafe {
                ptr.as_ptr()
                    .copy_from_nonoverlapping(array.ptr.as_ptr(), array.len())
            };

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

pub struct Box<'a, T> {
    ptr: NonNull<T>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> Box<'a, T> {
    pub fn new(arena: &'a mut Arena, value: T) -> Result<Self, Error> {
        arena.make(value)
    }

    pub fn new_with(arena: &'a mut Arena, f: impl FnOnce() -> T) -> Result<Self, Error> {
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

pub struct Array<'a, T> {
    ptr: NonNull<T>,
    len: u32,
    cap: u32,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> Array<'a, T> {
    pub fn new(mem: &'a Arena) -> Result<Self, Error> {
        mem.make_array(0)
    }

    pub fn with_capacity(mem: &'a Arena, cap: usize) -> Result<Self, Error> {
        mem.make_array(cap)
    }

    #[must_use]
    pub fn cap(&self) -> usize {
        self.cap as usize
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            unsafe { self.ptr.as_ptr().add(index).as_ref() }
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len() {
            unsafe { self.ptr.as_ptr().add(index).as_mut() }
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

    pub fn push(&mut self, arena: &'a mut Arena, value: T) -> Result<usize, Error> {
        if self.len == self.cap {
            arena.grow_array(self.cap as usize * 2, self)?;
        }

        debug_assert!(self.len < self.cap, "grow failed");

        unsafe { self.ptr.as_ptr().add(self.len as usize).write(value) };
        self.len += 1;

        Ok(self.len as usize)
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
    pub fn trim(&mut self, arena: &'a mut Arena) -> bool {
        arena.shrink_array(self.len(), self)
    }
}

impl<T> Deref for Array<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }
}

impl<T> DerefMut for Array<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }
}

impl<T> Index<usize> for Array<'_, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<T> IndexMut<usize> for Array<'_, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl<T> Drop for Array<'_, T> {
    fn drop(&mut self) {
        for i in 0..self.len() {
            unsafe { self.ptr.as_ptr().add(i).drop_in_place() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_heap() {
        todo!()
    }

    #[test]
    fn arena_stack() {
        todo!()
    }

    fn arena(arena: Arena) {
        // alloc

        // alloc_slice

        // reset
    }

    #[test]
    fn arena_box() {
        // make

        // make_with

        // drop

        // reset

        todo!()
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

        todo!()
    }
}
