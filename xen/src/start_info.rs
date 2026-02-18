/*
 * Copyright 2016-2017 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#[repr(C)]
pub struct start_info {
    /// "xen-<version>-<platform>"
    pub magic: [u8; 32],
    /// total pages allocated to this domain
    pub nr_pages: u64,
    /// machine address of struct shared_info
    pub shared_info: u64,
    /// SIF_xxx flags
    pub flags: u32,
    /// machine page number of shared page
    pub store_mfn: u64,
    /// event channel for store communication
    pub store_evtchn: u32,
    /// console (dom0/domU)
    pub console: start_info_console,
    /// virtual address of page directory
    pub pt_base: u64,
    /// number of bootstrap p.t. frames
    pub nr_pt_frames: u64,
    /// virtual address of page frame list
    pub mfn_list: u64,
    /// virtual address of pre-loaded module
    /// PFN of pre-loaded module if SIF_MOD_START_PFN set in flags
    pub mod_start: u64,
    /// size (bytes) of pre-loaded module
    pub mod_len: u64,
    /// guest command line
    pub cmd_line: [u8; 1024],
    /// PFN range here covers both page table and P->M table frames
    /// First PFN forming initial P->M table
    pub first_p2m_pfn: u64,
    /// number of PFNs forming initial P->M table
    pub nr_p2m_frames: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union start_info_console {
    pub domU: start_info_console_domU,
    pub dom0: start_info_console_dom0,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct start_info_console_domU {
    /// machine page number of console page
    pub mfn: u64,
    /// event channel for console page
    pub evtchn: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct start_info_console_dom0 {
    /// offset of struct console_info
    pub info_off: u32,
    /// size of struct console_info from start
    pub info_size: u32,
}
