/*
 * Copyright 2016-2017 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![no_std]
pub mod error;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "x86_64")]
pub type NativeXenHypercall = x86_64::X86XenHypercall;
#[cfg(target_arch = "aarch64")]
pub type NativeXenHypercall = aarch64::NativeXenHypercall;

pub use xen_bindings::*;

use crate::error::XenError;

/// Wrapper of a reference into a hypercall-safe buffer.
pub trait XenConstBuffer<T> {
    /// Get a hypercall-safe reference to the underlying data.
    fn as_hypercall_ptr(&self) -> *const T;
}

/// Wrapper of a mutable reference into a mutable hypercall-safe buffer.
pub trait XenMutBuffer<T> {
    /// Get a hypercall-safe mutable reference to the underlying data.
    fn as_hypercall_ptr(&mut self) -> *mut T;

    /// Update original reference with new data.
    unsafe fn update(&mut self);
}

/// Hypercall interface.
pub trait XenHypercall: Sized {
    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> Result<usize, XenError>;

    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall4(&self, cmd: usize, param: [usize; 4]) -> Result<usize, XenError> {
        self.hypercall5(cmd, [param[0], param[1], param[2], param[3], 0])
    }

    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall3(&self, cmd: usize, param: [usize; 3]) -> Result<usize, XenError> {
        self.hypercall4(cmd, [param[0], param[1], param[2], 0])
    }

    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall2(&self, cmd: usize, param: [usize; 2]) -> Result<usize, XenError> {
        self.hypercall3(cmd, [param[0], param[1], 0])
    }

    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall1(&self, cmd: usize, param: usize) -> Result<usize, XenError> {
        self.hypercall2(cmd, [param, 0])
    }

    /// Perform a raw hypercall.
    ///
    /// # SAFETY
    ///
    /// A hypercall has a externally defined behavior.
    unsafe fn hypercall0(&self, cmd: usize) -> Result<usize, XenError> {
        self.hypercall1(cmd, 0)
    }

    fn make_const_object<T: Copy>(&self, buffer: &T) -> Result<impl XenConstBuffer<T>, XenError>;

    fn make_mut_object<T: Copy>(&self, buffer: &mut T) -> Result<impl XenMutBuffer<T>, XenError>;

    // Slices needs some special handling as they are not Copy themselves
    // and a pointer to a slice doesn't point to its first element.

    fn make_const_slice<T: Copy + Sized>(
        &self,
        slice: &[T],
    ) -> Result<impl XenConstBuffer<T>, XenError>;

    fn make_mut_slice<T: Copy + Sized>(
        &self,
        slice: &mut [T],
    ) -> Result<impl XenMutBuffer<T>, XenError>;
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct DomId(pub u16);

impl DomId {
    pub const SELF: Self = Self(0x7FF0);
}

/// Constant xen buffer that passes the reference as-is.
pub(crate) struct DirectConstXenBuffer<'a, T>(&'a T);

impl<T> XenConstBuffer<T> for DirectConstXenBuffer<'_, T> {
    fn as_hypercall_ptr(&self) -> *const T {
        self.0
    }
}

pub(crate) struct DirectConstXenSlice<'a, T>(&'a [T]);

impl<T> XenConstBuffer<T> for DirectConstXenSlice<'_, T> {
    fn as_hypercall_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
}

/// Mutable xen buffer that passes the reference as-is.
pub(crate) struct DirectMutXenBuffer<'a, T>(&'a mut T);

impl<T> XenMutBuffer<T> for DirectMutXenBuffer<'_, T> {
    fn as_hypercall_ptr(&mut self) -> *mut T {
        self.0
    }

    unsafe fn update(&mut self) {
        // The buffer is passed as is, we don't need to bounce the changes.
    }
}

pub(crate) struct DirectMutXenSlice<'a, T>(&'a mut [T]);

impl<T> XenMutBuffer<T> for DirectMutXenSlice<'_, T> {
    fn as_hypercall_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr()
    }

    unsafe fn update(&mut self) {
        // The buffer is passed as is, we don't need to bounce the changes.
    }
}
