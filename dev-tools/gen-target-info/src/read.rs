use std::{io::BufRead, process};

use crate::{Cfgs, RustcTargetSpecs, TargetSpec};

fn get_cfgs(version: &str, target: &str) -> Cfgs {
    let mut cmd = process::Command::new("rustc");
    cmd.args([
        version,
        "-Zunstable-options",
        "--print",
        "cfg",
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

    let cfgs: Vec<String> = stdout.lines().map(|line| line.unwrap()).collect();
    Cfgs::parse(&cfgs)
}

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

    let mut spec: TargetSpec = serde_json::from_slice(&stdout).unwrap();
    spec.cfgs = get_cfgs("+1.63", target);
    spec
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

    let mut specs: RustcTargetSpecs = serde_json::from_slice(&stdout).unwrap();
    for (target, spec) in &mut specs.0 {
        spec.cfgs = get_cfgs("+nightly", target);
    }
    specs
}
