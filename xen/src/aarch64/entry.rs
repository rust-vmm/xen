/*
 * Copyright 2021-2022 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 *
 * Inspired by Philip Opperman's https://github.com/rust-osdev/bootloader
 */

extern crate xen_sys;

use core::arch::global_asm;

use crate::hypercall;

#[cfg(target_vendor = "xen")]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

global_asm!(include_str!("asm_macros.S"));
#[cfg(target_vendor = "xen")]
global_asm!(include_str!("head.S"));
global_asm!(include_str!("cache.S"));
global_asm!(include_str!("vector_table.S"));

#[export_name = "do_bad_mode"]
pub extern "C" fn do_bad_mode() -> ! {
    hypercall::console_io::write(b"bad mode\n");
    hypercall::sched_op::crash();

    #[allow(clippy::empty_loop)]
    loop {}
}

#[export_name = "do_trap_sync"]
pub extern "C" fn do_trap_sync() -> ! {
    hypercall::console_io::write(b"trap sync\n");
    hypercall::sched_op::crash();

    #[allow(clippy::empty_loop)]
    loop {}
}

#[export_name = "do_trap_irq"]
pub extern "C" fn do_trap_irq() -> ! {
    hypercall::console_io::write(b"trap irq\n");
    hypercall::sched_op::crash();

    #[allow(clippy::empty_loop)]
    loop {}
}

/// Defines the necessary functions and handlers to write a main function in
/// Rust
///
/// The function must have the signature `fn() -> !`.
///
/// This macro creates a function named `_start`, which Xen uses as the entry
/// point. This will perform the necessary startup actions for Xen before
/// handing control to your function. It additionally defines a panic handler
/// and a stack unwinder so that your application does not have to. The macro
/// ensures that the main function is the proper type.
///
/// Inspired by Philip Opperman's <https://github.com/rust-osdev/bootloader>
#[macro_export]
macro_rules! entry_point {
    ($entry:path) => {
        use core::panic::PanicInfo;

        #[export_name = "kernel_main"]
        pub extern "C" fn kernel_main() -> ! {
            // validate the signature of the program entry point
            let f: fn() -> Result<(), ()> = $entry;

            match f() {
                Ok(_) => hypercall::sched_op::poweroff(),
                Err(_) => {
                    hypercall::console_io::write(b"crash\n");
                    hypercall::sched_op::crash();
                }
            }

            loop {}
        }

        #[no_mangle]
        #[panic_handler]
        fn __impl_xen_panic(_info: &PanicInfo) -> ! {
            hypercall::sched_op::crash();
            loop {}
        }
    };
}
