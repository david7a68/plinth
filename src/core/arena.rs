//! Fixed-size arena allocator.

use std::{
    alloc::{alloc, Layout},
    cell::Cell,
    ptr::NonNull,
};

use allocator_api2::{
    alloc::{AllocError, Allocator},
    vec::Vec,
};

pub type Array<'a, T> = Vec<T, &'a Arena>;

pub struct Arena {
    root: *mut u8,
    stop: *mut u8,
    next: Cell<*mut u8>,
}

impl Arena {
    pub fn new(size: usize) -> Self {
        assert!(isize::try_from(size).is_ok(), "size exceeds isize::MAX");

        let layout = Layout::from_size_align(size, 8).unwrap();
        let root = unsafe { alloc(layout) };
        assert!(!root.is_null());

        Arena {
            root,
            stop: unsafe { root.add(layout.size()) },
            next: Cell::new(root),
        }
    }
}

impl Arena {
    pub fn capacity(&self) -> usize {
        unsafe { self.stop.offset_from(self.next.get()) as usize }
    }

    /// Resets the arena allocator.
    ///
    /// Any allocations made from the arena will be lost.
    pub fn reset(&mut self) {
        self.next.set(self.root);
    }
}

unsafe impl Allocator for Arena {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let align = self.next.get().align_offset(layout.align());
        let required = align + layout.size();

        debug_assert!(isize::try_from(required).is_ok());
        if required > self.capacity() {
            return Err(AllocError);
        }

        // SAFETY: `next` has been checked to have enough capacity for the
        // requested allocation.
        let ptr = unsafe {
            let ptr = self.next.get().add(align);
            self.next.set(ptr.add(layout.size()));
            NonNull::new_unchecked(ptr)
        };

        let slice = NonNull::slice_from_raw_parts(ptr, layout.size());

        Ok(slice)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let _ = layout;
        let _ = ptr;
        // no-op
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let is_last_alloc = unsafe { ptr.as_ptr().add(old_layout.size()) } == self.next.get();

        if is_last_alloc {
            let new_align = ptr.as_ptr().align_offset(new_layout.align()) != 0;
            if !new_align && self.capacity() >= new_layout.size() {
                let delta = new_layout.size().checked_sub(old_layout.size()).unwrap();
                self.next.set(unsafe { self.next.get().add(delta) });
                return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
            }
        }

        let mut mem = self.allocate(new_layout)?;
        copy_array(ptr, &mut mem, old_layout.size());

        Ok(mem)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let _ = old_layout;

        if ptr.as_ptr().align_offset(new_layout.align()) == 0 {
            Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()))
        } else {
            let mut mem = self.allocate(new_layout)?;
            copy_array(ptr, &mut mem, new_layout.size());
            Ok(mem)
        }
    }
}

fn copy_array(src: NonNull<u8>, dst: &mut NonNull<[u8]>, len: usize) {
    unsafe {
        dst.as_mut()
            .as_mut_ptr()
            .copy_from_nonoverlapping(src.as_ptr(), len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_heap() {
        let mut arena = Arena::new(1024);

        assert_eq!(unsafe { arena.root.offset_from(arena.stop) }, -1024);
        assert_eq!(arena.root, arena.next.get());

        arena.reset(); //  no-op here
        assert_eq!(arena.root, arena.next.get());

        let a = arena.allocate(Layout::array::<u8>(1024).unwrap()).unwrap();
        assert_eq!(unsafe { a.as_ref().as_ptr() }, arena.root);

        let b = arena.allocate(Layout::new::<usize>());
        b.unwrap_err();

        arena.reset();

        let c = arena.allocate(Layout::array::<u8>(3).unwrap()).unwrap();
        assert_eq!(unsafe { c.as_ref().as_ptr() }, arena.root);

        for i in 0..10 {
            // test aligning to layout
            let p = arena.allocate(Layout::new::<usize>()).unwrap();
            assert_eq!(unsafe { p.as_ref().as_ptr() }, unsafe {
                arena.root.add((i + 1) * std::mem::size_of::<usize>())
            });
        }

        // fill the rest of the arena
        let d = arena
            .allocate(Layout::array::<usize>(117).unwrap())
            .unwrap();

        assert_eq!(d.len(), Layout::array::<usize>(117).unwrap().size());

        let e = arena.allocate(Layout::new::<u8>());
        e.unwrap_err();
    }
}
