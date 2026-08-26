/*
 * Copyright 2016-2019 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */
#![allow(clippy::missing_safety_doc)]

use core::arch::asm;

use cty::c_long;

// x86_64 hypercalls are called at the address: 32 * HYPERCALL_NUM
#[repr(C)]
#[derive(Clone, Copy)]
struct hypercall_entry([u8; 32]);

// pages on x86_64 are 4096 bytes giving us 128 32-byte entries
extern "C" {
    static HYPERCALL_PAGE: [hypercall_entry; 128];
}

#[no_mangle]
pub unsafe fn hypercall_1(op: u32, a1: u64) -> c_long {
    let ret: c_long;
    let _ign1: u64;
    let addr = HYPERCALL_PAGE.as_ptr().offset(op as isize);

    asm!(
        "call {0}",
        in(reg) addr,
        inlateout("rax") addr => ret,
        inlateout("rdi") a1 => _ign1,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_2(op: u32, a1: u64, a2: u64) -> c_long {
    let ret: c_long;
    let _ign1: u64;
    let _ign2: u64;
    let addr = HYPERCALL_PAGE.as_ptr().offset(op as isize);

    asm!(
        "call {0}",
        in(reg) addr,
        inlateout("rax") addr => ret,
        inlateout("rdi") a1 => _ign1,
        inlateout("rsi") a2 => _ign2,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_3(op: u32, a1: u64, a2: u64, a3: u64) -> c_long {
    let ret: c_long;
    let _ign1: u64;
    let _ign2: u64;
    let _ign3: u64;
    let addr = HYPERCALL_PAGE.as_ptr().offset(op as isize);

    asm!(
      "call {0}",
      in(reg) addr,
      inlateout("rax") addr => ret,
      inlateout("rdi") a1 => _ign1,
      inlateout("rsi") a2 => _ign2,
      inlateout("rdx") a3 => _ign3,
      options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_4(op: u32, a1: u64, a2: u64, a3: u64, a4: u64) -> c_long {
    let ret: c_long;
    let _ign1: u64;
    let _ign2: u64;
    let _ign3: u64;
    let _ign4: u64;
    let addr = HYPERCALL_PAGE.as_ptr().offset(op as isize);

    asm!(
      "call {0}",
      in(reg) addr,
      inlateout("rax") addr => ret,
      inlateout("rdi") a1 => _ign1,
      inlateout("rsi") a2 => _ign2,
      inlateout("rdx") a3 => _ign3,
      inlateout("r10") a4 => _ign4,
      options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_5(op: u32, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> c_long {
    let ret: c_long;
    let _ign1: u64;
    let _ign2: u64;
    let _ign3: u64;
    let _ign4: u64;
    let _ign5: u64;
    let addr = HYPERCALL_PAGE.as_ptr().offset(op as isize);

    asm!(
      "call {0}",
      in(reg) addr,
      inlateout("rax") addr => ret,
      inlateout("rdi") a1 => _ign1,
      inlateout("rsi") a2 => _ign2,
      inlateout("rdx") a3 => _ign3,
      inlateout("r10") a4 => _ign4,
      inlateout("r9")  a5 => _ign5,
      options(nostack)
    );
    ret
}
