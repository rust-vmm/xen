// SPDX-License-Identifier: Apache-2.0
// Copyright © 2019 Intel Corporation

// Inspired by https://github.com/phil-opp/blog_os/blob/post-03/src/vga_buffer.rs
// from Philipp Oppermann

#[cfg(target_arch = "riscv64")]
pub mod uart_mmio;
#[cfg(target_arch = "aarch64")]
pub mod uart_pl011;

pub mod xen;

use core::fmt;

use atomic_refcell::AtomicRefCell;
#[cfg(target_arch = "x86_64")]
pub use uart_16550::SerialPort as Uart16550;
#[cfg(target_arch = "riscv64")]
pub use uart_mmio::UartMmio;
#[cfg(target_arch = "aarch64")]
pub use uart_pl011::Pl011 as UartPl011;

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::layout::map;
use crate::console::xen::XenConsole;

pub static DEFAULT: AtomicRefCell<Console> = AtomicRefCell::new(Console::None);

pub enum Console {
    None,
    Xen(XenConsole),
    #[cfg(target_arch = "x86_64")]
    Uart(Uart16550),
    #[cfg(target_arch = "aarch64")]
    Uart(UartPl011),
    #[cfg(target_arch = "riscv64")]
    Uart(UartMmio),
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Console::None => Ok(()),
            Console::Xen(xen_console) => xen_console.write_str(s),
            Console::Uart(serial_port) => serial_port.write_str(s),
        }
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        writeln!($crate::console::DEFAULT.borrow_mut(), $($arg)*).unwrap();
    }};
}

pub fn init() {
    // Try to initialize Xen PV console
    unsafe {
        if let Some(xen) = XenConsole::new() {
            *DEFAULT.borrow_mut() = Console::Xen(xen);
            return;
        }
    }

    // Fallback to UART
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // We use COM1 as it is the standard first serial port.
        *DEFAULT.borrow_mut() = Console::Uart(Uart16550::new(0x3f8));
    }

    // TODO: Fill from FDT?

    #[cfg(target_arch = "aarch64")]
    unsafe {
        *DEFAULT.borrow_mut() = Console::Uart(UartPl011::new(map::mmio::PL011_START));
    }

    #[cfg(target_arch = "riscv64")]
    {
        const SERIAL_PORT_ADDRESS: u64 = 0x1000_0000;
        *DEFAULT.borrow_mut() = Console::Uart(UartMmio::new(SERIAL_PORT_ADDRESS));
    }
}
