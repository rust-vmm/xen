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

use cty::c_long;

#[no_mangle]
pub unsafe fn hypercall_1(op: u32, a1: u64) -> c_long {
    let ret: c_long;

    asm!(
        "hvc 0xea1",
        in("x16") op as u64,
        in("x0") a1,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_2(op: u32, a1: u64, a2: u64) -> c_long {
    let ret: c_long;

    asm!(
        "hvc 0xea1",
        in("x16") op as u64,
        in("x0") a1,
        in("x1") a2,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_3(op: u32, a1: u64, a2: u64, a3: u64) -> c_long {
    let ret: c_long;

    asm!(
        "hvc 0xea1",
        in("x16") op as u64,
        in("x0") a1,
        in("x1") a2,
        in("x2") a3,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_4(op: u32, a1: u64, a2: u64, a3: u64, a4: u64) -> c_long {
    let ret: c_long;

    asm!(
        "hvc 0xea1",
        in("x16") op as u64,
        in("x0") a1,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

#[no_mangle]
pub unsafe fn hypercall_5(op: u32, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> c_long {
    let ret: c_long;

    asm!(
        "hvc 0xea1",
        in("x16") op as u64,
        in("x0") a1,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}
