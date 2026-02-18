// SPDX-License-Identifier: Apache-2.0
// Copyright © 2019 Intel Corporation
#![no_std]
#![no_main]
#![feature(sync_unsafe_cell)]
#![cfg_attr(target_arch = "riscv64", feature(riscv_ext_intrinsics))]

#[cfg(not(target_os = "none"))]
core::compile_error!("Unexpected non-freestanding target");

use core::panic::PanicInfo;

pub mod arch;
#[macro_use]
pub mod console;
pub mod bootinfo;
pub mod delay;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub mod fdt;
pub mod layout;
pub mod logger;
pub mod mem;
#[cfg(target_arch = "x86_64")]
pub mod pvh;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {info}");
    delay::stop_cpu()
}

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn rust64_start(pvh_info: &pvh::StartInfo) -> ! {
    arch::x86_64::mm::setup();
    arch::x86_64::sev::setup();
    arch::x86_64::idt::setup();

    console::init();
    logger::init();

    unsafe { oxerun_main(&(*pvh_info).into()) };

    delay::stop_cpu()
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub extern "C" fn rust64_start(x0: *const u8) -> ! {
    arch::aarch64::simd::setup_simd();
    arch::aarch64::paging::setup();

    // Use atomic operation before MMU enabled may cause exception, see https://www.ipshop.xyz/5909.html
    console::init();
    logger::init();

    let info = fdt::StartInfo::new(
        x0,
        Some(arch::aarch64::layout::map::dram::ACPI_START as u64),
        &crate::arch::aarch64::layout::MEM_LAYOUT[..],
        None,
    );

    unsafe { oxerun_main(&(*info).into()) };

    delay::stop_cpu()
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub extern "C" fn rust64_start(a0: u64, a1: *const u8) -> ! {
    use crate::bootinfo::{EntryType, Info, MemoryEntry};

    console::init();
    logger::init();

    log::info!("Starting on RV64 0x{:x} 0x{:x}", a0, a1 as u64,);

    let info = fdt::StartInfo::new(
        a1,
        None,
        &crate::arch::riscv64::layout::MEM_LAYOUT[..],
        Some(MemoryEntry {
            addr: 0x4000_0000,
            size: 2 << 20,
            entry_type: EntryType::Reserved,
        }),
    );

    for i in 0..info.num_entries() {
        let region = info.entry(i);
        log::info!(
            "Memory region {}MiB@0x{:x}",
            region.size / 1024 / 1024,
            region.addr
        );
    }

    unsafe { oxerun_main(&info) };

    delay::stop_cpu()
}

#[allow(improper_ctypes)]
unsafe extern "Rust" {
    unsafe fn oxerun_main(info: &bootinfo::BootInfo);
}
