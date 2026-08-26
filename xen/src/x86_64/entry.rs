/*
 * Copyright 2016-2017 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

extern crate xen_sys;

#[cfg(target_vendor = "xen")]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

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

        extern {
            static STACK_TOP: u64;
        }

        /// application name - XEN_ELFNOTE_GUEST_OS
        entry_point!(6, "asciz", concat!("\"", env!("CARGO_PKG_NAME"), "\""));
        /// application version - XEN_ELFNOTE_GUEST_VERSION
        entry_point!(7, "asciz", concat!("\"", env!("CARGO_PKG_VERSION"), "\""));
        /// PV loader - XEN_ELFNOTE_LOADER
        entry_point!(8, "asciz", "\"generic\"");
        /// setup HYPERCALL_PAGE - XEN_ELFNOTE_HYPERCALL_PAGE
        entry_point!(2, "quad", "HYPERCALL_PAGE");
        /// Xen ABI info - XEN_ELFNOTE_XEN_VERSION
        entry_point!(5, "asciz", "\"xen-3.0\"");
        /// Xen ABI features - XEN_ELFNOTE_FEATURES
        entry_point!(10, "asciz", "\"!writable_page_tables|pae_pgdir_above_4gb\"");
        /// Xen ABI PAE enabled - XEN_ELFNOTE_PAE_MODE
        entry_point!(9, "asciz", "\"yes\"");

        #[export_name = "_start"]
        pub extern "C" fn __impl_start() -> ! {
            // move to our own stack
            unsafe {
                asm!(
                    "mov rbp, {}",
                    "mov rsp, rbp",
                    in(reg) &STACK_TOP as *const u64,
                    options(nostack, nomem)
                );
            }

            // validate the signature of the program entry point
            let f: fn() -> Result<(), ()> = $entry;

            match f() {
                Ok(_) => hypercall::sched_op::poweroff(),
                Err(_) => {
                    hypercall::console_io::write(b"crash");
                    hypercall::sched_op::crash();
                }
            }
            loop {}
        }

        pub fn __rust_entry(_: *const xen_sys::start_info_t) -> ! {
            // validate the signature of the program entry point
            let f: fn() -> Result<(), ()> = $entry;

            match f() {
                Ok(_) => hypercall::sched_op::poweroff(),
                Err(_) => {
                    hypercall::console_io::write(b"crash");
                    hypercall::sched_op::crash();
                }
            }
            loop {}
        }

        #[panic_handler]
        fn __impl_xen_panic(_info: &PanicInfo) -> ! {
            hypercall::sched_op::crash();
            loop {}
        }
    };
    ($notetype:expr, $valty:expr, $value:expr) => {
        global_asm!(concat!(r#"
                .pushsection .note.Xen;
                .align 4;
                .long 2f - 1f;
                .long 4f - 3f;
                .long "#, $notetype, r#"
            1:  .asciz "Xen";
            2:  .align 4;
            3:  ."#, $valty, r#" "#, $value, r#"
            4:  .align 4;
                .popsection;
        "#));
    };
}
