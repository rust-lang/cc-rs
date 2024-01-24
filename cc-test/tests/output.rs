use std::fs;
use std::path::PathBuf;

#[test]
fn cargo_warnings_on() {
    if env!("TEST_WARNINGS_ON") == "0" {
        // in some cases we don't catch compiler warnings and turn them into cargo
        // instructions.
        return;
    }
    let (stdout, stderr) = load_output("warnings-on");
    assert!(stderr.is_empty());
    assert!(stdout.contains("cargo:warning="));
}

#[test]
fn cargo_warnings_off() {
    let (stdout, stderr) = load_output("warnings-off");
    assert!(stderr.is_empty());
    assert!(!stdout.contains("cargo:warning="));
}

#[test]
fn cargo_metadata_on() {
    let (stdout, stderr) = load_output("metadata-on");
    assert!(stderr.is_empty());
    assert!(stdout.contains("cargo:rustc-link-lib="));
    assert!(stdout.contains("cargo:rustc-link-search="));
}

#[test]
fn cargo_metadata_off() {
    let (stdout, stderr) = load_output("metadata-off");
    assert!(stderr.is_empty());

    // most of the instructions aren't currently used
    const INSTRUCTIONS: &[&str] = &[
        "cargo:rerun-if-changed=",
        "cargo:rerun-if-env-changed=",
        "cargo:rustc-cdylib-link-arg=",
        "cargo:rustc-cfg=",
        "cargo:rustc-env=",
        "cargo:rustc-flags=",
        "cargo:rustc-link-arg-benches=",
        "cargo:rustc-link-arg-bin=",
        "cargo:rustc-link-arg-bins=",
        "cargo:rustc-link-arg-examples=",
        "cargo:rustc-link-arg-tests=",
        "cargo:rustc-link-arg=",
        "cargo:rustc-link-lib=",
        "cargo:rustc-link-search=",
    ];
    for instr in INSTRUCTIONS {
        assert!(!stdout.contains(instr), "instruction present: {}", instr);
    }
}

#[track_caller]
fn load_output(action: &str) -> (String, String) {
    // these files are written by the `run_forked_capture_output` function in the
    // build script.
    let action_dir = PathBuf::from(env!("OUT_DIR")).join(action);
    let stdout = fs::read_to_string(action_dir.join("stdout")).unwrap();
    let stderr = fs::read_to_string(action_dir.join("stderr")).unwrap();
    println!("compile stdout: {:?}", stdout);
    println!("compile stderr: {:?}", stderr);
    (stdout, stderr)
}
