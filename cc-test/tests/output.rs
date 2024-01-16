use std::fs;
use std::path::PathBuf;

#[test]
fn cargo_warnings_on() {
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
    assert!(stdout.contains("rustc-link-lib="));
    assert!(stdout.contains("rustc-link-search="));
}

#[test]
fn cargo_metadata_off() {
    let (stdout, stderr) = load_output("metadata-off");
    assert!(stderr.is_empty());
    assert!(!stdout.contains("rustc-link-lib="));
    assert!(!stdout.contains("rustc-link-search="));
}

#[track_caller]
fn load_output(action: &str) -> (String, String) {
    // these files are written by the `run_forked_capture_output` function in the
    // build script.
    let action_dir = PathBuf::from(env!("OUT_DIR")).join(action);
    let stdout = fs::read_to_string(action_dir.join("stdout")).unwrap();
    let stderr = fs::read_to_string(action_dir.join("stderr")).unwrap();
    (stdout, stderr)
}
