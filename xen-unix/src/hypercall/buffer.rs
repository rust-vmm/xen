//! xencall device buffers
//!
//! Using the `privcmd` interface, Xen hypercalls needs the
//! parameters/structures to be placed into special memory mappings (onto
//! xencall device) in order to be "hypercall-safe" thus safely used as
//! addresses in the hypercalls.
//!
//! This module provides [UnixConstXenBuffer], [UnixMutXenBuffer],
//! [UnixConstXenSlice], [UnixMutXenSlice] to act as bounce-buffers (used by
//! [super::UnixXenHypercall]), the original value is copied onto a bounce
//! buffer and updated back by [XenMutBuffer::update].

extern crate std;

use std::{
    alloc::Layout,
    eprintln,
    marker::PhantomData,
    num::NonZeroUsize,
    os::fd::AsFd,
    ptr::{self, NonNull},
};

use nix::{
    errno::Errno,
    sys::mman::{self, MapFlags, ProtFlags},
};
use xen_sys::{XenConstBuffer, XenMutBuffer};

use super::UnixXenHypercall;

const PAGE_SIZE: usize = 4096;

impl UnixXenHypercall {
    /// Allocate a xencall (hypercall-safe) buffer
    fn alloc_xencall<'hyp, T>(&'hyp self, layout: Layout) -> Result<XenCallBuffer<'hyp, T>, Errno> {
        // TODO: It could be interesting to create a [std::alloc::Allocator] for these
        //       kind of objects. That way, we would be able to create several objects
        //       in a single page instead of allocating separate pages for each objects.

        assert!(
            layout.align() <= PAGE_SIZE,
            "Object cannot be aligned to page"
        );

        let size: usize = layout.size();

        if size == 0 {
            // ZST ?
            Ok(XenCallBuffer {
                interface: PhantomData::<&'hyp Self>,
                ptr: NonNull::dangling(),
                page_count: 0,
                length: 0,
            })
        } else {
            // Get the number of page to hold the object layout.
            let page_count = (size + (PAGE_SIZE - 1)) / PAGE_SIZE;
            let length = NonZeroUsize::new(page_count * PAGE_SIZE)
                .expect("Invalid size to page count convertion");

            // SAFETY: `addr` is defined as None
            //         `prot` and `flags` are legal values
            //         `length` is a multiple of page size
            let ptr: NonNull<T> = unsafe {
                mman::mmap(
                    None,
                    length,
                    ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                    MapFlags::MAP_SHARED,
                    self.hypercall_device.as_fd(),
                    0,
                )?
            }
            .cast();

            assert!(ptr.is_aligned(), "mmap gave us a non-aligned pointer");

            Ok(XenCallBuffer {
                interface: PhantomData::<&'hyp Self>,
                ptr,
                page_count,
                length: layout.size(),
            })
        }
    }

    pub(super) fn alloc_xencall_buffer<T>(&self) -> Result<XenCallBuffer<'_, T>, Errno> {
        self.alloc_xencall(Layout::new::<T>())
    }

    pub(super) fn alloc_xencall_slice<T>(&self, n: usize) -> Result<XenCallBuffer<'_, T>, Errno> {
        self.alloc_xencall(Layout::array::<T>(n).map_err(|_| Errno::E2BIG)?)
    }
}

pub struct XenCallBuffer<'hyp, T> {
    interface: PhantomData<&'hyp UnixXenHypercall>,
    ptr: NonNull<T>, // aligned
    page_count: usize,
    length: usize,
}

impl<T: Copy> XenCallBuffer<'_, T> {
    pub(super) unsafe fn read(&self) -> T {
        assert_eq!(self.length, size_of::<T>(), "invalid write operation");

        // SAFETY: Caller must ensure that data pointed can be read as T.
        //         `ptr` is properly aligned and valid for reads of T.
        self.ptr.read_volatile()
    }

    pub(super) fn write(&mut self, value: T) {
        assert_eq!(self.length, size_of::<T>(), "invalid write operation");

        // SAFETY: `ptr` is properly aligned and valid for writes of T.
        unsafe { self.ptr.write_volatile(value) }
    }

    pub(super) unsafe fn copy_to_slice(&self, slice: &mut [T]) {
        assert_eq!(
            size_of_val(slice),
            self.length,
            "Mismatched size of slice and buffer"
        );

        // SAFETY: `ptr` is properly aligned and valid for read of [T].
        ptr::copy_nonoverlapping(self.ptr.as_ptr(), slice.as_mut_ptr(), slice.len());
    }

    pub(super) fn copy_from_slice(&mut self, slice: &[T]) {
        assert_eq!(
            size_of_val(slice),
            self.length,
            "Mismatched size of slice and buffer"
        );

        // SAFETY: `ptr` is properly aligned and valid for writes of [T].
        unsafe { ptr::copy_nonoverlapping(slice.as_ptr(), self.ptr.as_ptr(), slice.len()) };
    }
}

impl<T> Drop for XenCallBuffer<'_, T> {
    fn drop(&mut self) {
        // if page_count is zero, self.bounce_ptr is dangling (and we are in the ZST
        // case)
        if self.page_count == 0 {
            return;
        }

        unsafe {
            if let Err(e) = mman::munmap(self.ptr.cast(), self.page_count * PAGE_SIZE) {
                // Best effort logging
                eprintln!(
                    "munmap({:p}, {}) failed ({})",
                    self.ptr,
                    self.page_count * PAGE_SIZE,
                    e.desc()
                );
            }
        };
    }
}

pub struct UnixConstXenBuffer<'a, 'hyp, T: Copy> {
    // As const objects are actually being copied they actually don't
    // need to hold a reference to their original counterpart.
    // Use a PhantomData to make the borrow checker happy.
    pub(super) original: PhantomData<&'a T>,
    pub(super) buffer: XenCallBuffer<'hyp, T>,
}

pub struct UnixMutXenBuffer<'a, 'hyp, T: Copy> {
    pub(super) original: &'a mut T,
    pub(super) buffer: XenCallBuffer<'hyp, T>,
}

impl<T: Copy> XenConstBuffer<T> for UnixConstXenBuffer<'_, '_, T> {
    fn as_hypercall_ptr(&self) -> *const T {
        self.buffer.ptr.as_ptr()
    }
}

impl<T: Copy> XenMutBuffer<T> for UnixMutXenBuffer<'_, '_, T> {
    fn as_hypercall_ptr(&mut self) -> *mut T {
        self.buffer.ptr.as_ptr()
    }

    unsafe fn update(&mut self) {
        // SAFETY: Caller must ensure that data pointed in `buffer` is valid for T.
        *self.original = self.buffer.read();
    }
}

pub struct UnixConstXenSlice<'a, 'hyp, T: Copy> {
    // As const objects are actually being copied they actually don't
    // need to hold a reference to their original counterpart.
    // Use a PhantomData to make the borrow checker happy.
    pub original: PhantomData<&'a [T]>,
    pub buffer: XenCallBuffer<'hyp, T>,
}

pub struct UnixMutXenSlice<'a, 'b, T: Copy> {
    pub original: &'a mut [T],
    pub buffer: XenCallBuffer<'b, T>,
}

impl<T: Copy> XenConstBuffer<T> for UnixConstXenSlice<'_, '_, T> {
    fn as_hypercall_ptr(&self) -> *const T {
        self.buffer.ptr.as_ptr()
    }
}

impl<T: Copy> XenMutBuffer<T> for UnixMutXenSlice<'_, '_, T> {
    fn as_hypercall_ptr(&mut self) -> *mut T {
        self.buffer.ptr.as_ptr()
    }

    unsafe fn update(&mut self) {
        // SAFETY: Caller must ensure that data pointed in `buffer` is valid for [T].
        self.buffer.copy_to_slice(self.original);
    }
}
