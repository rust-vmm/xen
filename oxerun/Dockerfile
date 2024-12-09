# SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
FROM debian:bookworm-slim
RUN <<EOF
#!/bin/sh

apt-get update -y
apt-get install -y \
    --no-install-suggests \
    --no-install-recommends \
    ca-certificates \
    wget \
    qemu-system-arm \
    ipxe-qemu \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    crossbuild-essential-arm64 \
    build-essential \
    flex \
    bison \
    ninja-build \
    make \
    gpg \
    gpg-agent \
    git \
    curl
rm -rf /var/lib/apt/lists/*
EOF

COPY <<EOF pgp@xen.org.asc
-----BEGIN PGP PUBLIC KEY BLOCK-----

mQENBEu7PVUBCADN/1+JPpAxp3fDk8jZQ3cUKA3W0maOlyI/4+nlDai1gh83m9CN
uGyY5kYLPBIR/sdG2hN5TVxTcE8qPCD9MivJXzOhBAmhQl0eXra0qmBBNu9kv+ZP
qtPORPg2Jch1zZL5jOMawIE0xARZPgu21rPKNJo7V+HejWAHh0/LfFxzzI8LZ1LJ
ACUuHEgfDJEi+u2wxDfjVaTO8HluNXm4TUIr16ExTx+61VDIE9qd3ikXkHgjp8xF
sH0qG5IfcFDTPx9L2Fyk0utTnuNW014P4R31n32U9OolFm1MyOzWrMwVBoTi34aE
nJRT6Aq/WaRfhjIWWkxhWnUgFbPPjMAkWL9fABEBAAG0WFhlbi5vcmcgWGVuIHRy
ZWUgY29kZSBzaWduaW5nIChzaWduYXR1cmVzIG9uIHRoZSB4ZW4gaHlwZXJ2aXNv
ciBhbmQgdG9vbHMpIDxwZ3BAeGVuLm9yZz6JAU0EEwECACAFAku7PVUCGwMGCwkI
BwMCBBUCCAMEFgIDAQIeAQIXgAAhCRCD/hTJV+gr2RYhBCPjIiwUX0R1+oBgp4P+
FMlX6CvZbMsH/jTs0LU/GouRrkjP71eOuEN/j127hYNbeDmTNclIz0PIhs0Ojsa4
hu6nPZkKuLniHBSgSo3MHwYYYzuiqaCFN4JtoRAZPtVpUPsGj/qgXeSgSBv0gKWk
TOIdpqAtlJCvo2QgI2qC68tRDlraLAyX1Ert/HGx8uAJ44a4RorhBmSJuWt0NiEu
LrlWoRJech/G7VO6gC0CLzaxmxwu3HuTfZqCYnnNEUgszAi+KShdZUMZkwffuFpd
6H8+pguFqIaGsXQIWBG1+FVCi9kdiN7Eme8Mxz75nKQnmEsP+d1p9uCzIui24me3
XGXPw9dq7cCDIXZv+iq5Si7zMPm76S74Yyc=
=h6NI
-----END PGP PUBLIC KEY BLOCK-----
EOF

RUN <<EOF
#!/bin/sh

set -ve

gpg --import pgp@xen.org.asc
EOF

RUN <<EOF
#!/bin/sh

set -ve

wget https://downloads.xenproject.org/release/xen/4.18.3/xen-4.18.3.tar.gz
EOF

RUN <<EOF
#!/bin/sh

set -ve

wget https://downloads.xenproject.org/release/xen/4.18.3/xen-4.18.3.tar.gz.sig
gpg --verify xen-4.18.3.tar.gz.sig xen-4.18.3.tar.gz
EOF

RUN <<EOF
#!/bin/sh

set -ve

tar xzf xen-4.18.3.tar.gz
EOF

RUN <<EOF
#!/bin/sh

set -ve

export WORKING_DIR="$(realpath `pwd`)"
echo WORKING_DIR=${WORKING_DIR}
cd ./xen-4.18.3/
export EXTRA_CFLAGS_XEN_CORE=" -Wno-array-bounds "
./configure --build=x86_64-unknown-linux-gnu --host=aarch64-linux-gnu \
  --disable-monitors \
  --disable-ocamltools \
  --disable-tools \
  --disable-docs \
  --disable-systemd \
  --enable-debug
make debug=y debug_symbols=y XEN_TARGET_ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu-
ls $(pwd)/dist/install/boot/
file $(pwd)/dist/install/boot/*
export XEN_KERNEL=$(realpath dist/install/boot/xen-4.18.3)
echo XEN_KERNEL=${XEN_KERNEL}
cp "${XEN_KERNEL}" "${WORKING_DIR}"/xen-4.18.3-kernel
EOF

#RUN <<EOF
##!/bin/sh

#curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused --location --silent --show-error --fail "https://sh.rustup.rs" | sh -s -- --default-toolchain none -y
#. "${HOME}/.cargo/env"
#rustup toolchain install --profile minimal nightly --target aarch64-linux-none
#EOF

COPY ./target/aarch64-xen-hvm/release/oxerun.bin /oxerun.bin

COPY <<"EOF" /run_qemu_oxerun.sh
#!/bin/sh

QEMU_OUTPUT=${QEMU_OUTPUT:-/qemu_output.stdout}
XEN_KERNEL=${XEN_KERNEL:-/xen-4.18.3-kernel}
DOM0_UNIKERNEL=${DOM0_UNIKERNEL:-/oxerun.bin}

echo QEMU_OUTPUT="${QEMU_OUTPUT}"
echo XEN_KERNEL="${XEN_KERNEL}"
echo DOM0_UNIKERNEL="${DOM0_UNIKERNEL}"

if [ ! -f "${XEN_KERNEL}" ]; then
  printf 'XEN_KERNEL value %s does not exist!\n' "${XEN_KERNEL}" 1>&2
fi

if [ ! -f "${DOM0_UNIKERNEL}" ]; then
  printf 'DOM0_UNIKERNEL value %s does not exist!\n' "${DOM0_UNIKERNEL}" 1>&2
fi

qemu-system-aarch64 \
  -D qemu.log \
  -d guest_errors \
  -machine virt,virtualization=on,gic-version=3 \
  -cpu cortex-a57 \
  -chardev file,id=char0,path="${QEMU_OUTPUT}" \
  -serial chardev:char0 \
  -monitor none \
  -display none \
  -m 8192 \
  -smp 4 \
  -kernel "${XEN_KERNEL}" \
  -append 'dom0_mem=4G loglvl=none guest_loglvl=none sync_console=true' \
  -device guest-loader,addr=0x80000000,kernel="${DOM0_UNIKERNEL}",bootargs='dom0_mem=4G loglvl=none guest_loglvl=none sync_console=true'

cat > /tmp/expected_output <<DOF
 Xen 4.18.3
oxerun hello world
DOF

if ! diff -w -u /tmp/expected_output "${QEMU_OUTPUT}"> /dev/null 2>&1; then
  diff -w -u /tmp/expected_output "${QEMU_OUTPUT}"
  exit 1
fi

exit 0
EOF

CMD ["/bin/sh", "/run_qemu_oxerun.sh"]
