use std::process;

use crate::RustcTargetSpecs;

pub fn get_target_specs_from_json() -> RustcTargetSpecs {
    let mut cmd = process::Command::new("rustc");
    cmd.args([
        "+nightly",
        "-Zunstable-options",
        "--print",
        "all-target-specs-json",
    ])
    .stdout(process::Stdio::piped());

    let process::Output { status, stdout, .. } = cmd.output().unwrap();

    if !status.success() {
        panic!("{:?} failed with non-zero exit status: {}", cmd, status)
    }

    serde_json::from_slice(&stdout).unwrap()
}
