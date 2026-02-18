#![no_main]
#![no_std]
#![feature(sync_unsafe_cell)]

use oxerun::{
    bootinfo::{self, Info},
    println,
};

#[unsafe(no_mangle)]
fn oxerun_main(info: &bootinfo::BootInfo) {
    println!("Hello World (oxerun v2) !");

    println!("boot protocol: {}", info.name());
    println!(
        "cmdline: {}",
        str::from_utf8(info.cmdline()).unwrap_or("(non-utf8)")
    );
    println!("memory layout");
    for descriptor in info.memory_layout() {
        println!(
            "- {}\t{:08x?}: {:?}",
            descriptor.name,
            (descriptor.range)(),
            descriptor.attribute
        );
    }

    println!("memory map");
    for i in 0..info.num_entries() {
        if let Some(entry) = info.entry(i) {
            println!(
                "- {:012x}..{:012x} {:?}",
                entry.addr,
                entry.addr + entry.size,
                entry.entry_type
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{__cpuid, __get_cpuid_max, CpuidResult};

        use oxerun::arch::x86_64::sev::SEV_STATUS;
        use x86_64::registers::model_specific::Msr;

        let raw_hypervisor_leaf = {
            let CpuidResult {
                eax: _,
                ebx,
                ecx,
                edx,
            } = __cpuid(0x4000_0000);
            [ebx.to_ne_bytes(), ecx.to_ne_bytes(), edx.to_ne_bytes()]
        };

        let hypervisor_string = str::from_utf8(raw_hypervisor_leaf.as_flattened());

        println!("Hypervisor: {}", hypervisor_string.unwrap_or("(non-utf8)"));

        if __get_cpuid_max(0x8000_0000).0 > 0x8000_001f {
            println!("-- SME CPUID --");
            let sev_leaf = __cpuid(0x8000_001f);

            println!("SME status : {}", sev_leaf.eax & (1 << 0) > 0);
            println!("SEV status : {}", sev_leaf.eax & (1 << 1) > 0);
            println!("C-Bit: {}", sev_leaf.ebx & 0x3f);
            println!("SEV-ES: {}", sev_leaf.eax & (1 << 3) > 0);

            println!("-- SEV_STATUS MSR --");

            println!("SEV_ENABLED: {}", SEV_STATUS & (1 << 0) > 0);
            println!("SEV_ES_ENABLED: {}", SEV_STATUS & (1 << 1) > 0);
        }

        println!("IA32_APIC_BASE: {:08x}", Msr::new(0x1b).read());
    }
}
