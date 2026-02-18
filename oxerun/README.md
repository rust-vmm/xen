[![Build status](https://gitlab.com/cardoe/oxerun/badges/master/pipeline.svg)](https://gitlab.com/cardoe/oxerun/commits/master)

# Example project using Rust for Xen unikernels

This is an example build of Rust building a full unikernel for Xen.

You need to use nightly Rust compiler.

## Building for x86_64:

Using Rust 1.92 nightly build.

```shell
# cargo build -p oxerun -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec --target x86_64-xen-hvm.json --bin hello
```

And to generate assember files in target/x86_64-xen-pv/{release|debug}/deps/

```shell
# RUSTFLAGS="--emit asm -C llvm-args=-x86-asm-syntax=intel" cargo build -p oxerun -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec --target x86_64-xen-hvm.json --bin hello
```

## Building for aarch64:

```shell
# cargo build --target aarch64-xen-hvm.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
```

And to generate assember files in target/aarch64-xen-hvm/{release|debug}/deps/

```shell
# RUSTFLAGS="--emit asm" cargo build --target aarch64-xen-hvm.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
```

On aarch64 Xen accept a binary type unikernel and as such the ELF resulting from the cargo builds needs to be converted as follow:

```shell
# aarch64-linux-gnu-objcopy target/aarch64-xen-hvm/release/oxerun -O binary target/aarch64-xen-hvm/release/oxerun.bin
```

## Running the unikernel on QEMU:

```shell
$ qemu-system-aarch64 --version
QEMU emulator version 6.0.94 (v6.1.0-rc4-2-gecf5e5ec1f44)
Copyright (c) 2003-2021 Fabrice Bellard and the QEMU Project developers
$
$ qemu-system-aarch64 \
	-machine virt,virtualization=on \
	-cpu cortex-a57 -serial mon:stdio \
	-device virtio-net-pci,netdev=net0				\
	-netdev user,id=net0,hostfwd=tcp::8022-:22			\
	-device virtio-scsi-pci \
	-drive file=debian-buster-arm64,index=0,id=hd0,if=none,format=raw \
	-device scsi-hd,drive=hd0 \
	-display none \
	-m 8192 -smp 4\
	-kernel $(PATH_TO_XEN)/xen/xen \
	-device guest-loader,addr=0x80000000,kernel=$(PATH_TO_XEN_SYS)/xen-sys/target/aarch64-xen-hvm/release/oxerun.bin
```

In the above the unikernel is loaded at address 0x80000000, which means that Xen will relocate the binary image to 0x60000000
before jumping to it.  The aarch64 LD script for the unikernel, i.e aarch64-xen-hvm.ld, links the image at address 0x60000000.  At
this time the unikernel is run on a flat memory model without MMU support.
