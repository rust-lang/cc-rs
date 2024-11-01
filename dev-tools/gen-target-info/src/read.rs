use std::process;

use crate::{RustcTargetSpecs, TargetSpec};

pub fn get_targets_msrv() -> Vec<u8> {
    let mut cmd = process::Command::new("rustc");
    cmd.args(["+1.63", "--print", "target-list"]);
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::inherit());

    let process::Output { status, stdout, .. } = cmd.output().unwrap();

    if !status.success() {
        panic!("{:?} failed with non-zero exit status: {}", cmd, status)
    }

    stdout
}

pub fn get_target_spec_from_msrv(target: &str) -> TargetSpec {
    let mut cmd = process::Command::new("rustc");
    cmd.args([
        "+1.63",
        "-Zunstable-options",
        "--print",
        "target-spec-json",
        "--target",
        target,
    ]);
    cmd.env("RUSTC_BOOTSTRAP", "1");
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::inherit());

    let process::Output { status, stdout, .. } = cmd.output().unwrap();

    if !status.success() {
        panic!("{:?} failed with non-zero exit status: {}", cmd, status)
    }

    serde_json::from_slice(&stdout).unwrap()
}

pub fn get_target_specs_from_json() -> RustcTargetSpecs {
    let mut cmd = process::Command::new("rustc");
    cmd.args([
        "+nightly",
        "-Zunstable-options",
        "--print",
        "all-target-specs-json",
    ]);
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::inherit());

    let process::Output { status, stdout, .. } = cmd.output().unwrap();

    if !status.success() {
        panic!("{:?} failed with non-zero exit status: {}", cmd, status)
    }

    serde_json::from_slice(&stdout).unwrap()
}
