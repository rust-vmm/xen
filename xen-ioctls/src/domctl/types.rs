/*
 * Copyright 2021-22 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![allow(dead_code)]
#![allow(non_upper_case_globals)]

#[cfg(target_arch = "aarch64")]
use crate::aarch64::types::*;
#[cfg(target_arch = "x86_64")]
use crate::x86_64::types::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "xen_domctl_interface_version_0x18")] {
        pub const XEN_DOMCTL_INTERFACE_VERSION: u32 = 0x18;
    } else if #[cfg(feature = "xen_domctl_interface_version_0x17")] {
        pub const XEN_DOMCTL_INTERFACE_VERSION: u32 = 0x17;
    } else if #[cfg(feature = "xen_domctl_interface_version_0x16")] {
        pub const XEN_DOMCTL_INTERFACE_VERSION: u32 = 0x16;
    } else if #[cfg(feature = "xen_domctl_interface_version_0x15")] {
        pub const XEN_DOMCTL_INTERFACE_VERSION: u32 = 0x15;
    } else {
        pub const XEN_DOMCTL_INTERFACE_VERSION: u32 = 0x18;
    }
}

pub const XEN_DOMINF_dying: u32 = 0b1;
pub const XEN_DOMINF_hvm_guest: u32 = 0b10;
pub const XEN_DOMINF_shutdown: u32 = 0b100;
pub const XEN_DOMINF_paused: u32 = 0b1000;
pub const XEN_DOMINF_blocked: u32 = 0b10000;
pub const XEN_DOMINF_running: u32 = 0b100000;
pub const XEN_DOMINF_debugged: u32 = 0b1000000;
pub const XEN_DOMINF_xs_domain: u32 = 0b10000000;
pub const XEN_DOMINF_hap: u32 = 0b100000000;
pub const XEN_DOMINF_shutdownmask: u32 = 255;
pub const XEN_DOMINF_shutdownshift: u32 = 16;
pub const SHUTDOWN_crash: u32 = 3;

// xen/include/public/domctl.h::struct xen_domctl_getdomaininfo
// sizeof(struct xen_domctl_get_domaininfo) == 120
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct XenDomctlGetDomainInfo {
    pub domain: u16,
    pad: u16,
    pub flags: u32,
    pub tot_pages: U64Aligned,
    pub max_pages: U64Aligned,
    pub outstanding_pages: U64Aligned,
    pub shr_pages: U64Aligned,
    pub paged_pages: U64Aligned,
    pub shared_info_frame: U64Aligned,
    pub cpu_time: U64Aligned,
    pub nr_online_vcpus: u32,
    pub max_vcpu_id: u32,
    pub ssidref: u32,
    pub handle: [u8; 16],
    pub cpupool: u32,
    pub gpaddr_bits: u8,
    pad2: [u8; 7],
    pub arch_config: XenArchDomainconfig,
}

pub const XEN_DOMCTL_createdomain: u32 = 1;
pub const XEN_DOMCTL_destroydomain: u32 = 2;
pub const XEN_DOMCTL_pausedomain: u32 = 3;
pub const XEN_DOMCTL_unpausedomain: u32 = 4;
pub const XEN_DOMCTL_getdomaininfo: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone)]
pub union XenDomctlPayload {
    pub domaininfo: XenDomctlGetDomainInfo,
    pad: [u8; 128],
}

#[repr(C)]
#[derive(Copy, Clone)]
// xen/include/public/domctl.h::struct xen_domctl
pub struct XenDomctl {
    pub cmd: u32,
    pub interface_version: u32, /* XEN_DOMCTL_INTERFACE_VERSION */
    pub domain: u16,
    pub pad: [u16; 3],
    pub u: XenDomctlPayload,
}
