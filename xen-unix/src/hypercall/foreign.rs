extern crate std;

use core::{ffi::c_void, marker::PhantomData, num::NonZeroUsize};
use std::{
    ffi::{c_int, c_uint, c_ulong},
    io,
    os::fd::{AsFd, AsRawFd},
    ptr::NonNull,
};

use nix::{
    ioc, ioctl_readwrite_bad,
    sys::mman::{self, MapFlags, ProtFlags},
};
use xen_sys::DomId;

use crate::hypercall::UnixXenHypercall;

const PAGE_SIZE: usize = 4096;

pub struct ForeignMap<'a> {
    interface: PhantomData<&'a UnixXenHypercall>,
    pub addr: NonNull<c_void>,
    pub length: usize,
}

impl ForeignMap<'_> {
    pub fn empty() -> Self {
        Self {
            interface: PhantomData,
            addr: NonNull::dangling(),
            length: 0,
        }
    }
}

#[repr(C)]
struct PrivcmdMmapBatchV2 {
    num: c_uint,
    dom: DomId,
    addr: u64,
    arr: *const c_ulong,
    err: *mut c_int,
}
ioctl_readwrite_bad!(
    mmap_batch_v2,
    ioc!(0, b'P', 4, size_of::<PrivcmdMmapBatchV2>()),
    PrivcmdMmapBatchV2
);

#[repr(C)]
struct PrivCmdMmapResource {
    dom: DomId,
    map_type: u32,
    id: u32,
    idx: u32,
    num: u64,
    addr: u64,
}
ioctl_readwrite_bad!(
    mmap_resource,
    ioc!(0, b'P', 7, size_of::<PrivCmdMmapResource>()),
    PrivCmdMmapResource
);

impl UnixXenHypercall {
    pub fn foreign_map<'a>(&'a self, domid: DomId, pfns: &[c_ulong]) -> io::Result<ForeignMap<'a>> {
        let Some(length) = NonZeroUsize::new(pfns.len() * PAGE_SIZE) else {
            return Ok(ForeignMap::empty());
        };

        let addr: NonNull<c_void> = unsafe {
            mman::mmap(
                None,
                length,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                self.privcmd_device.as_fd(),
                0,
            )?
        }
        .cast();

        let mut err: c_int = 0;

        let mut param = PrivcmdMmapBatchV2 {
            num: pfns.len() as u32,
            dom: domid,
            addr: addr.addr().get() as u64,
            arr: pfns.as_ptr(),
            err: &raw mut err,
        };

        if let Err(e) = unsafe { mmap_batch_v2(self.privcmd_device.as_raw_fd(), &mut param) } {
            unsafe { mman::munmap(addr, length.get()).ok() };
            return Err(e.into());
        }

        Ok(ForeignMap {
            interface: PhantomData::<&'a Self>,
            addr,
            length: length.get(),
        })
    }

    pub fn resource_map<'a>(
        &'a self,
        domid: DomId,
        map_type: u32,
        id: u32,
        idx: u32,
        num: usize,
    ) -> io::Result<ForeignMap<'a>> {
        let Some(length) = NonZeroUsize::new(num * PAGE_SIZE) else {
            return Ok(ForeignMap::empty());
        };

        let addr: NonNull<c_void> = unsafe {
            mman::mmap(
                None,
                length,
                ProtFlags::PROT_READ,
                MapFlags::MAP_SHARED,
                self.privcmd_device.as_fd(),
                0,
            )?
        }
        .cast();

        let mut param = PrivCmdMmapResource {
            num: num as _,
            dom: domid,
            addr: addr.addr().get() as u64,
            map_type,
            id,
            idx,
        };

        if let Err(e) = unsafe { mmap_resource(self.privcmd_device.as_raw_fd(), &mut param) } {
            unsafe { mman::munmap(addr, length.get()).ok() };
            return Err(e.into());
        }

        Ok(ForeignMap {
            interface: PhantomData::<&'a Self>,
            addr,
            length: length.get(),
        })
    }
}

impl Drop for ForeignMap<'_> {
    fn drop(&mut self) {
        // if page_count is zero, self.bounce_ptr is dangling
        if self.length == 0 {
            return;
        }

        unsafe {
            if let Err(e) = mman::munmap(self.addr.cast(), self.length) {
                // Best effort logging
                eprintln!(
                    "munmap({:p}, {}) failed ({})",
                    self.addr,
                    self.length,
                    e.desc()
                );
            }
        };
    }
}
