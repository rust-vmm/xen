/*
 * Copyright 2016-2017 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![allow(non_camel_case_types, clippy::missing_safety_doc)]

use xen_bindings::bindings::{
    CONSOLEIO_read, CONSOLEIO_write, SCHEDOP_shutdown, SCHEDOP_yield, __HYPERVISOR_console_io,
    __HYPERVISOR_sched_op,
};

use crate::{error::XenError, XenConstBuffer, XenHypercall, XenMutBuffer};

/// SCHEDOP_ defines from public/sched.h
#[derive(Debug)]
pub enum SchedOp {
    /// SCHEDOP_yield
    r#yield,
    /// SCHEDOP_block
    block,
    /// SCHEDOP_shutdown
    shutdown,
    /// SCHEDOP_poll
    poll,
    /// SCHEDOP_remote_shutdown
    remote_shutdown,
    /// SCHEDOP_shutdown_code
    shutdown_code,
    /// SCHEDOP_watchdog
    watchdog,
    /// SCHEDOP_pin_override
    pin_override,
}

/// CONSOLEIO_ defines from public/xen.h
#[derive(Debug)]
pub enum ConsoleIO {
    /// CONSOLEIO_write
    Write,
    /// CONSOLEIO_read
    Read,
}

pub fn console_io<H: XenHypercall>(
    hyp: &H,
    mode: ConsoleIO,
    buf: &mut [u8],
) -> Result<i64, XenError> {
    let len = buf.len();

    match mode {
        ConsoleIO::Write => {
            let hyp_buffer = hyp.make_const_slice(buf)?;

            let ret = unsafe {
                hyp.hypercall3(
                    __HYPERVISOR_console_io as usize,
                    [
                        CONSOLEIO_write as usize,
                        len,
                        hyp_buffer.as_hypercall_ptr().addr(),
                    ],
                )?
            };

            Ok(ret as i64)
        }
        ConsoleIO::Read => {
            let mut hyp_buffer = hyp.make_mut_slice(buf)?;

            let ret = unsafe {
                hyp.hypercall3(
                    __HYPERVISOR_console_io as usize,
                    [
                        CONSOLEIO_read as usize,
                        len,
                        hyp_buffer.as_hypercall_ptr().addr(),
                    ],
                )?
            };
            unsafe { hyp_buffer.update() };

            Ok(ret as i64)
        }
    }
}

pub unsafe fn sched_op<H: XenHypercall>(hyp: &H, mode: SchedOp, data: u32) -> Result<(), XenError> {
    match mode {
        SchedOp::r#yield => {
            hyp.hypercall1(__HYPERVISOR_sched_op as usize, SCHEDOP_yield as usize)?;
        }
        SchedOp::shutdown => {
            let reason = hyp.make_const_object(&data)?;

            hyp.hypercall2(
                __HYPERVISOR_sched_op as usize,
                [SCHEDOP_shutdown as usize, reason.as_hypercall_ptr().addr()],
            )?;
        }
        _ => unimplemented!(),
    };

    Ok(())
}
