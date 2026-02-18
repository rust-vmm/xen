//! x86 Xen interface.
//!
//! See [X86XenHypercall].

use core::arch::x86_64::{CpuidResult, __cpuid};

use crate::{
    error::{parse_hypercall_return, XenError},
    DirectConstXenBuffer, DirectConstXenSlice, DirectMutXenBuffer, DirectMutXenSlice,
    XenConstBuffer, XenHypercall, XenMutBuffer,
};

/// x86 HVM hypercall interface.
///
/// This interface can only be used when running in kernel-mode (CPL0).
/// Trying to use it in user-mode (CPL3) will lead to #GP.
///
/// Using this interface with a incorrect hypercall kind will likely lead to
/// incorrect behavior.
#[derive(Clone, Copy, Debug)]
pub enum X86XenHypercall {
    /// Intel hypercalls `vmcall`
    Intel,
    /// AMD hypercalls `vmmcall`
    Amd,
}

#[macro_export]
macro_rules! native_hypercall {
    ($h:expr, $($t:tt)*) => {
        match $h {
            Self::Intel => core::arch::asm!("vmcall", $($t)*),
            Self::Amd => core::arch::asm!("vmmcall", $($t)*),
        }
    };
}

/// From xen/include/public/arch-x86/cpuid.h
///
/// For compatibility with other hypervisor interfaces, the Xen cpuid leaves
/// can be found at the first otherwise unused 0x100 aligned boundary starting
/// from 0x40000000.
const XEN_CPUID_FIRST_LEAF: u32 = 0x4000_0000;

/// Finds Xen leaf using a provided CPUID function.
///
/// # SAFETY
///
/// Safety conditions are inherited from provided `cpuid_func`.
pub unsafe fn find_xen_leaves2(cpuid_func: unsafe fn(u32) -> CpuidResult) -> Option<u32> {
    for base in (XEN_CPUID_FIRST_LEAF..(XEN_CPUID_FIRST_LEAF + 0x10000)).step_by(0x100) {
        let cpuid = cpuid_func(base);

        if match cpuid {
            CpuidResult {
                eax,
                ebx: 0x566e6558, // "XenV"
                ecx: 0x65584d4d, // "MMXe"
                edx: 0x4d4d566e, // "nVMM"
            } => eax - base >= 2,
            _ => false,
        } {
            return Some(base);
        }
    }

    None
}

/// Finds Xen CPUID leaf using native CPUID instruction.
///
/// # SAFETY
///
/// This function assumes CPUID instruction is available.
pub unsafe fn find_xen_leaves() -> Option<u32> {
    find_xen_leaves2(__cpuid)
}

impl X86XenHypercall {
    /// Detect if we are running under Xen, and if so, use the appropriate
    /// native hypercall interface. Uses CPUID instruction.
    ///
    /// # SAFETY
    ///
    /// This function assumes CPUID instruction is available.
    pub unsafe fn new() -> Option<Self> {
        find_xen_leaves()?;

        // We are running under Xen.
        match __cpuid(0) {
            // GenuineIntel
            CpuidResult {
                eax: _,
                ebx: 0x756e6547,
                ecx: 0x6c65746e,
                edx: 0x49656e69,
            } => Some(Self::Intel),
            // AuthenticAMD
            CpuidResult {
                eax: _,
                ebx: 0x68747541,
                ecx: 0x444d4163,
                edx: 0x69746e65,
            } => Some(Self::Amd),
            // TODO: Centaur, Hygon, ...
            _ => None,
        }
    }
}

/*
 * Based on x86 Hypercall ABI (64-bits)
 *
 * Hypercall index: RAX
 * Parameters: RDI, RSI, RDX, R10, R8
 * Result: RAX
 */

impl XenHypercall for X86XenHypercall {
    #[inline(always)]
    unsafe fn hypercall5(&self, cmd: usize, param: [usize; 5]) -> Result<usize, XenError> {
        let output: isize;

        native_hypercall!(
            self,
            inlateout("rax") cmd => output,
            inlateout("rdi") param[0] => _,
            inlateout("rsi") param[1] => _,
            inlateout("rdx") param[2] => _,
            inlateout("r10") param[3] => _,
            inlateout("r8") param[4] => _,
        );

        parse_hypercall_return(output)
    }

    fn make_const_object<T: Copy>(&self, buffer: &T) -> Result<impl XenConstBuffer<T>, XenError> {
        Ok(DirectConstXenBuffer(buffer))
    }

    fn make_mut_object<T: Copy>(&self, buffer: &mut T) -> Result<impl XenMutBuffer<T>, XenError> {
        Ok(DirectMutXenBuffer(buffer))
    }

    fn make_const_slice<T: Copy + Sized>(
        &self,
        slice: &[T],
    ) -> Result<impl XenConstBuffer<T>, XenError> {
        Ok(DirectConstXenSlice(slice))
    }

    fn make_mut_slice<T: Copy + Sized>(
        &self,
        slice: &mut [T],
    ) -> Result<impl XenMutBuffer<T>, XenError> {
        Ok(DirectMutXenSlice(slice))
    }
}
