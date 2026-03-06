mod support;
use std::{env, path::PathBuf, process::Command};

use crate::support::Test;

#[test]
fn main() {
    env_propagates_to_subprocesses();
}

fn env_propagates_to_subprocesses() {
    if !cfg!(target_os = "macos") {
        return;
    }

    // Get developer dir before changing the global environment.
    let developer_dir = xcode_select_developer_dir();
    // Set developer dir to something bogus.
    unsafe { env::set_var("DEVELOPER_DIR", "foo") };

    let test = Test::gnu();

    // But set it properly for sub-invocations of `xcrun`.
    test.gcc()
        .env("DEVELOPER_DIR", developer_dir)
        .file("foo.c")
        .compile("foo");

    // If this fails, we failed to propagate `Build::env` to `xcrun`.
}

fn xcode_select_developer_dir() -> PathBuf {
    let output = Command::new("xcode-select")
        .arg("--print-path")
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("could not invoke `xcode-select`");
    }
    let output = String::from_utf8(output.stdout).unwrap();
    PathBuf::from(output.trim_end())
}
