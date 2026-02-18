// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2022 Akira Moroo
// Copyright (C) 2025 Vates SAS - Teddy Astie

use core::ptr::NonNull;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

pub fn map_4k_frame<T>(pfn: u64, #[allow(unused)] encrypted: bool) -> Option<NonNull<T>> {
    #[cfg(target_arch = "x86_64")]
    {
        use crate::arch::x86_64::mm::map_frame;
        use ::x86_64::{PhysAddr, structures::paging::PhysFrame};

        let vaddr = unsafe {
            map_frame(
                PhysFrame::from_start_address(PhysAddr::new(pfn << 12))
                    .expect("Invalid mapping pfn"),
                encrypted,
            )
        }?;

        NonNull::new(vaddr.as_mut_ptr())
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        /* Assume flat memory model */
        NonNull::new((pfn << 12) as *mut T)
    }
}
