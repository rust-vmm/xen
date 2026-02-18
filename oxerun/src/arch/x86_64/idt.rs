use core::{arch::naked_asm, cell::SyncUnsafeCell};

use x86_64::{VirtAddr, structures::idt::InterruptDescriptorTable};

use crate::arch::x86_64::sev::ghcb_vc_handler;

pub static IDT: SyncUnsafeCell<InterruptDescriptorTable> =
    SyncUnsafeCell::new(InterruptDescriptorTable::new());

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CpuRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[unsafe(naked)]
unsafe extern "C" fn raw_ghcb_vc_handler() {
    naked_asm!(include_str!("isr.s"), options(att_syntax), EXCEPTION_HANDLER = sym ghcb_vc_handler)
}

pub fn setup() {
    let idt = unsafe { &mut *IDT.get() };

    unsafe {
        idt.vmm_communication_exception
            .set_handler_addr(VirtAddr::new(raw_ghcb_vc_handler as *const () as u64));
    }

    idt.load();
}
