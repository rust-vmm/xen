// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2022 Akira Moroo

use enum_dispatch::enum_dispatch;

use crate::{layout::MemoryDescriptor};

// Common data needed for all boot paths
#[enum_dispatch(BootInfo)]
pub trait Info {
    // Name of for this boot protocol
    fn name(&self) -> &str;
    // Starting address of the Root System Descriptor Pointer
    fn rsdp_addr(&self) -> Option<u64> {
        None
    }
    // Address/size of FDT used for booting
    fn fdt_reservation(&self) -> Option<MemoryEntry> {
        None
    }
    // The kernel command line (not including null terminator)
    fn cmdline(&self) -> &[u8];
    // Methods to access the Memory map
    fn num_entries(&self) -> usize;
    fn entry(&self, idx: usize) -> Option<MemoryEntry>;
    // 
    fn memory_layout(&self) -> &'static [MemoryDescriptor];
    // MMIO address space that can be used for PCI BARs if needed
    fn pci_bar_memory(&self) -> Option<MemoryEntry> {
        None
    }
}

#[derive(Clone, Copy)]
pub struct MemoryEntry {
    pub addr: u64,
    pub size: u64,
    pub entry_type: EntryType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntryType {
    Ram,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    Bad,
    VendorReserved,
    Persistent,
}

#[enum_dispatch]
#[derive(Clone, Copy)]
pub enum BootInfo {
    #[cfg(target_arch = "x86_64")]
    Pvh(crate::pvh::StartInfo),
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    Fdt(crate::fdt::StartInfo<'static>)
}
