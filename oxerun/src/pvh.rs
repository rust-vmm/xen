// SPDX-License-Identifier: Apache-2.0
// Copyright 2020 Google LLC

use core::mem::size_of;

use crate::{
    bootinfo::{EntryType, Info, MemoryEntry},
    layout::MemoryDescriptor,
};

pub const XEN_HVM_MEMMAP_TYPE_RAM: u32 = 1;
pub const XEN_HVM_MEMMAP_TYPE_RESERVED: u32 = 2;
pub const XEN_HVM_MEMMAP_TYPE_ACPI: u32 = 3;
pub const XEN_HVM_MEMMAP_TYPE_NVS: u32 = 4;
pub const XEN_HVM_MEMMAP_TYPE_UNUSABLE: u32 = 5;
pub const XEN_HVM_MEMMAP_TYPE_DISABLED: u32 = 6;
pub const XEN_HVM_MEMMAP_TYPE_PMEM: u32 = 7;

// Structures from xen/include/public/arch-x86/hvm/start_info.h
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StartInfo {
    magic: [u8; 4],
    version: u32,
    flags: u32,
    nr_modules: u32,
    modlist_paddr: u64,
    cmdline_paddr: u64,
    rsdp_paddr: u64,
    memmap_paddr: u64,
    memmap_entries: u32,
    _pad: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct MemMapEntry {
    addr: u64,
    size: u64,
    entry_type: u32,
    _pad: u32,
}

impl From<MemMapEntry> for MemoryEntry {
    fn from(value: MemMapEntry) -> Self {
        Self {
            addr: value.addr,
            size: value.size,
            entry_type: match value.entry_type {
                XEN_HVM_MEMMAP_TYPE_RAM => EntryType::Ram,
                XEN_HVM_MEMMAP_TYPE_RESERVED => EntryType::Reserved,
                XEN_HVM_MEMMAP_TYPE_ACPI => EntryType::AcpiReclaimable,
                XEN_HVM_MEMMAP_TYPE_NVS => EntryType::AcpiNvs,
                XEN_HVM_MEMMAP_TYPE_UNUSABLE => EntryType::Bad,
                XEN_HVM_MEMMAP_TYPE_DISABLED => EntryType::Bad,
                XEN_HVM_MEMMAP_TYPE_PMEM => EntryType::Persistent,
                _ => EntryType::Reserved, // Unknown
            },
        }
    }
}

impl Info for StartInfo {
    fn name(&self) -> &str {
        "PVH Boot Protocol"
    }
    fn rsdp_addr(&self) -> Option<u64> {
        Some(self.rsdp_paddr)
    }
    fn cmdline(&self) -> &[u8] {
        if self.cmdline_paddr == 0 {
            return &[];
        }
        let start = self.cmdline_paddr as *const u8;
        let mut size: usize = 0;
        while unsafe { start.add(size).read() } != 0 {
            size += 1;
        }
        unsafe { core::slice::from_raw_parts(start, size) }
    }
    fn num_entries(&self) -> usize {
        // memmap_paddr and memmap_entries only exist in version 1 or later
        if self.version < 1 || self.memmap_paddr == 0 {
            return 0;
        }
        self.memmap_entries as usize
    }
    fn entry(&self, idx: usize) -> Option<MemoryEntry> {
        if idx >= self.num_entries() {
            return None;
        }

        let ptr = self.memmap_paddr as *const MemMapEntry;
        Some(MemoryEntry::from(unsafe { *ptr.add(idx) }))
    }
    fn memory_layout(&self) -> &'static [MemoryDescriptor] {
        &crate::arch::x86_64::layout::MEM_LAYOUT[..]
    }
}

// The PVH Boot Protocol starts at the 32-bit entrypoint to our firmware.
unsafe extern "C" {
    fn ram32_start();
}

// The kind/name/desc of the PHV ELF Note are from xen/include/public/elfnote.h.
// This is the "Physical entry point into the kernel".
const XEN_ELFNOTE_PHYS32_ENTRY: u32 = 18;
type Name = [u8; 4];
type Desc = unsafe extern "C" fn();

// We make sure our ELF Note has an alignment of 4 for maximum compatibility.
// Some software (QEMU) calculates padding incorectly if alignment != 4.
#[repr(C, packed(4))]
struct Note {
    name_size: u32,
    desc_size: u32,
    kind: u32,
    name: Name,
    desc: Desc,
}

// This is: ELFNOTE(Xen, XEN_ELFNOTE_PHYS32_ENTRY, .quad ram32_start)
#[unsafe(link_section = ".note")]
#[used]
static PVH_NOTE: Note = Note {
    name_size: size_of::<Name>() as u32,
    desc_size: size_of::<Desc>() as u32,
    kind: XEN_ELFNOTE_PHYS32_ENTRY,
    name: *b"Xen\0",
    desc: ram32_start,
};
