# SPDX-License-Identifier: Apache-2.0
# Copyright 2020 Google LLC
# Copyright 2025 Teddy Astie - Vates SAS

.section .text32, "ax"
.global ram32_start
.code32

ram32_start:
    # Stash the PVH start_info struct in %rdi.
    movl %ebx, %edi

setup_page_tables:
    # First L2 entry identity maps [0, 2 MiB)
    movl $0b10000011, (L2_TABLE) # huge (bit 7), writable (bit 1), present (bit 0)
    # First L3 entry points to L2 table
    movl $L2_TABLE, %eax
    orb  $0b00000011, %al # writable (bit 1), present (bit 0)
    movl %eax, (L3_TABLE)
    # First L4 entry points to L3 table
    movl $L3_TABLE, %eax
    orb  $0b00000011, %al # writable (bit 1), present (bit 0)
    movl %eax, (L4_TABLE)

sev_check:
    # Check GHCB/SEV-ES through start_info.flags & SIF_HVM_GHCB.
    mov 8(%edi), %edx
    btl $5, %edx
    jnc no_ghcb

use_ghcb:
    # Use GHCB protocol instead.
    movl $0xc0010130, %ecx # MSR_AMD64_SEV_GHCB
    rdmsr
    # C-bit is in EAX[31:24]
    shr $24, %eax
    mov %eax, %ebx
    jmp sev_bit_known

no_ghcb:
    # Check CPUID highest leaf
    movl $0x80000000, %eax
    cpuid
    cmpl $0x8000001f, %eax
    jb enable_paging

    # Check for SEV support
    movl $0x8000001f, %eax
    cpuid
    btl $1, %eax
    jnc enable_paging

sev_bit_known:
    # Check if SEV is enabled
    movl $0xc0010131, %ecx # MSR_AMD64_SEV
    rdmsr
    movl %eax, (SEV_STATUS)
    btl $0, %eax # MSR_AMD64_SEV_ENABLED_BIT
    jnc enable_paging

    movl %ebx, %ecx
    andl $0x3f, %ecx # Get C-bit position
    subl $0x20, %ecx
    movl $1, %ebx
    shll %cl, %ebx

    # %ebx contains high part of C-bit mask
    # We assume that C-bit is over the 32-bits mark.
    movl %ebx, (MEMORY_ENCRYPT_FLAG + 4)

    # Inject C-bit to pagetables
    leal (L2_TABLE), %eax
    orl %ebx, 4(%eax)
    leal (L3_TABLE), %eax
    orl %ebx, 4(%eax)
    leal (L4_TABLE), %eax
    orl %ebx, 4(%eax)

enable_paging:
    # Load page table root into CR3
    movl $L4_TABLE, %eax
    movl %eax, %cr3

    # Set CR4.PAE (Physical Address Extension)
    movl %cr4, %eax
    orb  $0b00100000, %al # Set bit 5
    movl %eax, %cr4
    # Set EFER.LME (Long Mode Enable)
    movl $0xC0000080, %ecx
    rdmsr
    orb  $0b00000001, %ah # Set bit 8
    wrmsr
    # Set CRO.PG (Paging)
    movl %cr0, %eax
    orl  $(1 << 31), %eax
    movl %eax, %cr0

jump_to_64bit:
    # We are now in 32-bit compatibility mode. To enter 64-bit mode, we need to
    # load a 64-bit code segment into our GDT.
    lgdtl GDT64_PTR
    # Initialize the stack pointer (Rust code always uses the stack)
    movl $stack_end, %esp
    # Set segment registers to a 64-bit segment.
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %gs
    movw %ax, %fs
    movw %ax, %ss
    # Set CS to a 64-bit segment and jump to 64-bit Rust code.
    # PVH start_info is in %rdi, the first paramter of the System V ABI.
    ljmpl $0x08, $rust64_start

.code64