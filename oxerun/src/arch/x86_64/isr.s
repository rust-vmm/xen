# Based on Linux ISR handler
.code64
pushq %rdi
pushq %rsi
pushq %rdx
pushq %rcx
pushq %rax
pushq %r8
pushq %r9
pushq %r10
pushq %r11
pushq %rbx
pushq %rbp
pushq %r12
pushq %r13
pushq %r14
pushq %r15

# Call handler with CpuRegs
movq %rsp, %rdi
# Error code is second parameter
movq 120(%rsp), %rsi
call {EXCEPTION_HANDLER}

/* Restore regs */
popq %r15
popq %r14
popq %r13
popq %r12
popq %rbp
popq %rbx
popq %r11
popq %r10
popq %r9
popq %r8
popq %rax
popq %rcx
popq %rdx
popq %rsi
popq %rdi

# Remove error code and return
addq    $8, %rsp

iretq
