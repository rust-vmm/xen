use core::{ptr::NonNull, sync::atomic::AtomicU32};

use volatile::{VolatileFieldAccess, VolatilePtr};

use crate::io::ring::XenRing;

pub mod ring;

#[repr(C)]
#[derive(VolatileFieldAccess)]
pub struct XenConsInterface {
    in_buffer: [u8; 1024],
    out_buffer: [u8; 2048],
    in_cons: u32,
    in_prod: u32,
    out_cons: u32,
    out_prod: u32,
}

impl XenConsInterface {
    pub unsafe fn new(ptr: NonNull<Self>) -> VolatilePtr<'static, Self> {
        unsafe { VolatilePtr::new(ptr) }
    }

    pub fn to_ring<'a>(interface: VolatilePtr<'a, Self>) -> XenRing<'a> {
        // SAFETY: Atomic pointers are built from valid pointers.
        unsafe {
            XenRing {
                ring: interface.out_buffer().as_slice(),
                cons: AtomicU32::from_ptr(interface.out_cons().as_raw_ptr().as_ptr()),
                prod: AtomicU32::from_ptr(interface.out_prod().as_raw_ptr().as_ptr()),
            }
        }
    }
}
