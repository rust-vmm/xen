/*
 * Copyright 2021-22 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![allow(clippy::missing_safety_doc)]

use core::arch::asm;

use crate::{
    error::{parse_hypercall_return, XenError},
    DirectConstXenBuffer, DirectConstXenSlice, DirectMutXenBuffer, DirectMutXenSlice,
    XenConstBuffer, XenHypercall, XenMutBuffer,
};

#[derive(Clone, Copy, Debug)]
pub struct NativeXenHypercall;

impl NativeXenHypercall {
    pub unsafe fn new() -> Option<Self> {
        Some(Self)
    }
}

impl XenHypercall for NativeXenHypercall {
    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> Result<usize, XenError> {
        // TODO
        Err(XenError::Other("TODO"))
    }

    fn make_const_object<T: Copy>(&self, buffer: &T) -> Result<impl XenConstBuffer<T>, XenError> {
        Ok(DirectConstXenBuffer(buffer))
    }

    fn make_mut_object<T: Copy>(&self, buffer: &mut T) -> Result<impl XenMutBuffer<T>, XenError> {
        Ok(DirectMutXenBuffer(buffer))
    }

    fn make_const_slice<T: Copy + Sized>(
        &self,
        slice: &[T],
    ) -> Result<impl XenConstBuffer<T>, XenError> {
        Ok(DirectConstXenSlice(slice))
    }

    fn make_mut_slice<T: Copy + Sized>(
        &self,
        slice: &mut [T],
    ) -> Result<impl XenMutBuffer<T>, XenError> {
        Ok(DirectMutXenSlice(slice))
    }
}
