// SPDX-License-Identifier: Apache-2.0
// Copyright 2020 Google LLC
// Copyright 2025 Vates SAS

/*
 * Memory layout
 *
 * 0M-1G(-2M) : Identity
 * 1G(-2M)-1G : L1 mappings
 */

use core::cell::SyncUnsafeCell;

use x86_64::{
    PhysAddr, VirtAddr,
    instructions::tlb,
    registers::control::Cr3,
    structures::paging::{
        PageSize, PageTable, PageTableFlags, PageTableIndex, PhysFrame, Size2MiB, Size4KiB,
    },
};

#[unsafe(no_mangle)]
static L4_TABLE: SyncUnsafeCell<PageTable> = SyncUnsafeCell::new(PageTable::new());
#[unsafe(no_mangle)]
static L3_TABLE: SyncUnsafeCell<PageTable> = SyncUnsafeCell::new(PageTable::new());
#[unsafe(no_mangle)]
static L2_TABLE: SyncUnsafeCell<PageTable> = SyncUnsafeCell::new(PageTable::new());
#[unsafe(no_mangle)]
static L1_TABLE: SyncUnsafeCell<PageTable> = SyncUnsafeCell::new(PageTable::new());

#[unsafe(no_mangle)]
pub static mut MEMORY_ENCRYPT_FLAG: PageTableFlags = PageTableFlags::empty();

/// Position of the L1 mappings in the L2 table.
/// 511 means 2M just below 1G
const L1_INDEX: PageTableIndex = PageTableIndex::new(511);

pub fn setup() {
    // SAFETY: This function is idempontent and only writes to static memory and
    // CR3. Thus, it is safe to run multiple times or on multiple threads.
    // A SyncUnsafeCell pointer is never null.
    let (l4, l3, l2, l1) = unsafe {
        (
            &mut *L4_TABLE.get(),
            &mut *L3_TABLE.get(),
            &mut *L2_TABLE.get(),
            &mut *L1_TABLE.get(),
        )
    };
    let pt_flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | unsafe { MEMORY_ENCRYPT_FLAG };

    let mut next_addr = PhysAddr::zero();
    for (pos, l2e) in l2.iter_mut().enumerate() {
        if pos == L1_INDEX.into() {
            l2e.set_addr(phys_addr_linear(l1), pt_flags);
        } else {
            l2e.set_addr(next_addr, pt_flags | PageTableFlags::HUGE_PAGE);
        }

        next_addr += Size2MiB::SIZE;
    }

    // Point L3 at L2
    l3[0].set_addr(phys_addr_linear(l2), pt_flags);
    // Point L4 at L3
    l4[0].set_addr(phys_addr_linear(l3), pt_flags);

    // Point Cr3 at L4
    let (cr3_frame, cr3_flags) = Cr3::read();
    let l4_frame = PhysFrame::from_start_address(phys_addr_linear(l4)).unwrap();
    if cr3_frame != l4_frame {
        unsafe { Cr3::write(l4_frame, cr3_flags) };
    }
}

fn phys_addr_linear<T>(virt_addr: *const T) -> PhysAddr {
    PhysAddr::new(virt_addr.addr() as u64)
}

pub unsafe fn map_frame(frame: PhysFrame<Size4KiB>, encrypted: bool) -> Option<VirtAddr> {
    let l1 = unsafe { &mut *L1_TABLE.get() };
    // Find a spare L1 entry
    let (index, entry) = l1.iter_mut().enumerate().find(|(_, l1e)| l1e.is_unused())?;
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    if encrypted {
        flags |= unsafe { MEMORY_ENCRYPT_FLAG };
    }

    entry.set_frame(frame, flags);

    Some(VirtAddr::new(
        u64::from(L1_INDEX) * Size2MiB::SIZE + (index as u64) * Size4KiB::SIZE,
    ))
}

pub unsafe fn unmap_frame(va: VirtAddr) -> Result<(), ()> {
    if u32::from(va.p4_index()) != 0 && u32::from(va.p3_index()) != 0 && va.p2_index() != L1_INDEX {
        return Err(());
    }

    unsafe {
        let l1 = &mut *L1_TABLE.get();
        l1[va.p1_index()].set_unused();
        tlb::flush(va);
    };
    Ok(())
}
