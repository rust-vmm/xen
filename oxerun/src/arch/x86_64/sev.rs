use core::{
    arch::asm,
    cell::SyncUnsafeCell,
    ptr::{NonNull, null_mut},
    sync::atomic::{AtomicPtr, Ordering},
};

use volatile::{VolatileFieldAccess, VolatilePtr};
use x86_64::{PhysAddr, registers::model_specific::Msr, structures::paging::PhysFrame};

use crate::arch::x86_64::{idt::CpuRegs, mm};

const SEV_STATUS_MSR: u32 = 0xc001_0131;
const GHCB_MSR: u32 = 0xc001_0130;

const GHCB_EXIT_REQ: u64 = 0x100;

const VMEXIT_CPUID: u64 = 0x72;
const VMEXIT_MSR: u64 = 0x7c;
const VMEXIT_VMMCALL: u64 = 0x81;

const VMMCALL_INST_LEN: u64 = 0x3;
const CPUID_INST_LEN: u64 = 0x2;
const MSR_INST_LEN: u64 = 0x2;

// Based on Enarx SEV code

/// GHCB Save Area
#[derive(Debug, Copy, Clone, VolatileFieldAccess)]
#[repr(C)]
pub struct GhcbSaveArea {
    reserved1: [u8; 203],
    pub cpl: u8,
    reserved2: [u8; 300],
    pub rax: u64,
    reserved3: [u8; 264],
    pub rcx: u64,
    pub rdx: u64,
    pub rbx: u64,
    reserved4: u64,
    pub rbp: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    reserved5: [u8; 16],
    pub sw_exit_code: u64,
    pub sw_exit_info1: u64,
    pub sw_exit_info2: u64,
    pub sw_scratch: u64,
    reserved6: [u8; 56],
    pub xcr0: u64,
    pub valid_bitmap: [u8; 16],
    pub x87state_gpa: u64,
    reserved7: [u8; 1016],
}

/// GHCB
#[derive(Debug, Copy, Clone, VolatileFieldAccess)]
#[repr(C, align(4096))]
pub struct Ghcb {
    pub save_area: GhcbSaveArea,
    pub shared_buffer: [u8; 2032],
    reserved1: [u8; 10],
    pub protocol_version: u16,
    pub ghcb_usage: u32,
}

impl Ghcb {
    const fn new() -> Self {
        Self {
            save_area: GhcbSaveArea {
                reserved1: [0; _],
                cpl: 0,
                reserved2: [0; _],
                rax: 0,
                reserved3: [0; _],
                rcx: 0,
                rdx: 0,
                rbx: 0,
                reserved4: 0,
                rbp: 0,
                rsi: 0,
                rdi: 0,
                r8: 0,
                r9: 0,
                r10: 0,
                r11: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
                reserved5: [0; _],
                sw_exit_code: 0,
                sw_exit_info1: 0,
                sw_exit_info2: 0,
                sw_scratch: 0,
                reserved6: [0; _],
                xcr0: 0,
                valid_bitmap: [0; _],
                x87state_gpa: 0,
                reserved7: [0; _],
            },
            shared_buffer: [0; _],
            reserved1: [0; _],
            protocol_version: 0,
            ghcb_usage: 0,
        }
    }
}

static GHCB_PAGE: SyncUnsafeCell<Ghcb> = SyncUnsafeCell::new(Ghcb::new());
static GHCB: AtomicPtr<Ghcb> = AtomicPtr::new(null_mut());

#[unsafe(no_mangle)]
pub static mut SEV_STATUS: u64 = 0;

pub fn get_ghcb() -> Option<VolatilePtr<'static, Ghcb>> {
    // SAFETY: We always give a valid pointer and there is no SMT.
    unsafe {
        Some(VolatilePtr::new(NonNull::new(
            GHCB.load(Ordering::Relaxed),
        )?))
    }
}

pub fn setup() {
    // SAFETY: No other thread exist at this time.
    if unsafe { SEV_STATUS } & 0x2 == 0 {
        // No SEV-ES
        return;
    }

    // SAFETY: GHCB_PAGE is identity mapped.
    unsafe {
        GHCB.store(
            mm::map_frame(
                PhysFrame::from_start_address_unchecked(PhysAddr::new(GHCB_PAGE.get() as u64)),
                false,
            )
            .unwrap()
            .as_mut_ptr(),
            Ordering::Relaxed,
        );
    }

    // SAFETY: We just initialized ghcb so it can't be None.
    let ghcb = unsafe { get_ghcb().unwrap_unchecked() };

    ghcb.protocol_version().write(0x1); // SEV-ES Version 1
}

#[inline]
pub unsafe fn vmgexit_msr(value: u64) -> u64 {
    let mut ghcb_msr = Msr::new(GHCB_MSR);

    unsafe {
        ghcb_msr.write(value);
        asm!("rep; vmmcall", options(nostack, preserves_flags));
        ghcb_msr.read()
    }
}

fn ghcb_call() -> u64 {
    unsafe { vmgexit_msr(GHCB_PAGE.get() as u64) }
}

fn sev_es_terminate(reason: u16) -> ! {
    unsafe {
        vmgexit_msr(GHCB_EXIT_REQ | ((reason as u64) << 16));
        asm!("cli;hlt");
        core::hint::unreachable_unchecked()
    };
}

pub fn ghcb_handle_cpuid(regs: &mut CpuRegs, ghcb: VolatilePtr<'static, Ghcb>) {
    let save_area = ghcb.save_area();

    save_area.rax().write(regs.rax);
    save_area.rcx().write(regs.rcx);

    save_area.sw_exit_code().write(VMEXIT_CPUID);
    save_area.sw_exit_info1().write(0);
    save_area.sw_exit_info2().write(0);

    // TODO: valid bitmap, xcr0 special case

    let result = ghcb_call();

    // TODO: handle result

    regs.rax = save_area.rax().read();
    regs.rbx = save_area.rbx().read();
    regs.rcx = save_area.rcx().read();
    regs.rdx = save_area.rdx().read();

    regs.rip += CPUID_INST_LEN;
}

pub fn ghcb_handle_msr(regs: &mut CpuRegs, ghcb: VolatilePtr<'static, Ghcb>) {
    let save_area = ghcb.save_area();
    let opcode = unsafe { (regs.rip as *const u16).read_unaligned() };

    let wrmsr = match opcode.to_ne_bytes() {
        [0x0F, 0x30] => true,  /* WRMSR */
        [0x0F, 0x32] => false, /* RDMSR */
        _ => sev_es_terminate(1),
    };

    save_area.sw_exit_code().write(VMEXIT_MSR);
    save_area.sw_exit_info1().write(wrmsr as _);
    save_area.rcx().write(regs.rcx as u32 as _);

    if wrmsr {
        save_area.rdx().write(regs.rdx as u32 as _);
        save_area.rax().write(regs.rax as u32 as _);
    }

    let result = ghcb_call();

    if save_area.sw_exit_info1().read() == 1 {
        sev_es_terminate(3);
    }

    if !wrmsr {
        regs.rdx = save_area.rdx().read() as u32 as _;
        regs.rax = save_area.rax().read() as u32 as _;
    }

    regs.rip += MSR_INST_LEN;
}

pub fn ghcb_handle_vmmcall(regs: &mut CpuRegs, ghcb: VolatilePtr<'static, Ghcb>) {
    let save_area = ghcb.save_area();

    save_area.rax().write(regs.rax);
    save_area.rdi().write(regs.rdi);
    save_area.rsi().write(regs.rsi);
    save_area.r8().write(regs.r8);
    save_area.r9().write(regs.r9);
    save_area.r10().write(regs.r10);
    save_area.r11().write(regs.r11);
    save_area.r12().write(regs.r12);

    save_area.cpl().write(0);

    save_area.sw_exit_code().write(VMEXIT_VMMCALL);
    save_area.sw_exit_info1().write(0);
    save_area.sw_exit_info2().write(0);

    let result = ghcb_call();

    // TODO: handle result

    regs.rax = save_area.rax().read();
    regs.rdi = save_area.rdi().read();
    regs.rsi = save_area.rsi().read();
    regs.r8 = save_area.r8().read();
    regs.r9 = save_area.r9().read();
    regs.r10 = save_area.r10().read();
    regs.r11 = save_area.r11().read();
    regs.r12 = save_area.r12().read();

    regs.rip += VMMCALL_INST_LEN;
}

pub extern "C" fn ghcb_vc_handler(regs: &mut CpuRegs, error_code: u64) {
    let Some(ghcb) = get_ghcb() else {
        sev_es_terminate(2);
    };

    match error_code {
        VMEXIT_CPUID => ghcb_handle_cpuid(regs, ghcb),
        VMEXIT_MSR => ghcb_handle_msr(regs, ghcb),
        VMEXIT_VMMCALL => ghcb_handle_vmmcall(regs, ghcb),
        _ => {}
    }
}

pub fn is_sev_guest() -> bool {
    unsafe { SEV_STATUS != 0 }
}

pub fn is_sev_es_guest() -> bool {
    unsafe { (SEV_STATUS & 0x2) != 0 }
}
