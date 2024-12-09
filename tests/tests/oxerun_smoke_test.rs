// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::{
    io::BufRead,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread::sleep,
    time::{Duration, Instant},
};

#[test]
fn test_run_aarch64_smoke_test_in_qemu() {
    let repo_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../oxerun");
    let cargo_bin_path = Path::new(env!("CARGO"));
    //cargo build --target aarch64-xen-hvm.json -Zbuild-std=core
    // -Zbuild-std-features=compiler-builtins-mem --release

    dbg!(Command::new(&cargo_bin_path)
        .args([
            "build",
            "--target",
            "./aarch64-xen-hvm.json",
            "-Zbuild-std=core",
            "-Zbuild-std-features=compiler-builtins-mem",
            "--release"
        ])
        .current_dir(&repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap());

    // aarch64-linux-gnu-objcopy target/aarch64-xen-hvm/release/oxerun -O binary
    // target/aarch64-xen-hvm/release/oxerun.bin
    dbg!(Command::new("aarch64-linux-gnu-objcopy")
        .args([
            "./target/aarch64-xen-hvm/release/oxerun",
            "-O",
            "binary",
            "./target/aarch64-xen-hvm/release/oxerun.bin"
        ])
        .current_dir(&repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap());

    // podman build . | tail -n 1
    let output = dbg!(Command::new("podman")
        .args(["build", "."])
        .current_dir(&repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .unwrap());
    let id = dbg!(output.stdout.lines().last().unwrap().unwrap());

    // podman run $img
    let mut child = dbg!(Command::new("podman")
        .args(["run", &id])
        .current_dir(&repo_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap());
    // wait 30 secs max
    let start = Instant::now();
    while Instant::now() - start <= Duration::from_secs(30) {
        if child.try_wait().is_ok() {
            let output = child.wait_with_output().unwrap();
            eprintln!("output:\n{:?}", &output);
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
            return;
        }
        sleep(Duration::from_millis(250));
    }
    if child.try_wait().is_err() {
        if let Err(err) = child.kill() {
            eprintln!("Could not wait for podmun run: {}", err);
        }
        let output = child.wait_with_output().unwrap();
        eprintln!("output:\n{:?}", &output);
        eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
