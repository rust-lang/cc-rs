//! Test for issue #504: BusyBox ar compatibility
//! BusyBox ar doesn't support the `-s` flag, so we need to fall back to ranlib

#![allow(clippy::disallowed_methods)]

use crate::support::Test;

mod support;

#[test]
fn busybox_ar_fallback() {
    // Use standard test setup with proper cc-shim
    let test = Test::gnu();
    test.shim("ranlib");  // Add ranlib shim for fallback testing

    // Override ar with a BusyBox-like version that fails on -s flag
    // But still creates the archive file for other operations
    let ar_script = if cfg!(windows) {
        r#"@echo off
REM Check for -s flag - fail if present
for %%a in (%*) do (
    if "%%a"=="s" (
        echo BusyBox ar: unknown option -- s >&2
        exit /b 1
    )
)

REM Create the archive file (last argument is typically the output file)
REM Get last argument
set "LAST_ARG="
for %%a in (%*) do set "LAST_ARG=%%a"
REM Create an empty file to simulate archive creation
if not "%LAST_ARG%"=="" type nul > "%LAST_ARG%"
exit /b 0
"#
    } else {
        r#"#!/bin/sh
# Check for -s flag - fail if present
for arg in "$@"; do
    if [ "$arg" = "s" ]; then
        echo "BusyBox ar: unknown option -- s" >&2
        exit 1
    fi
done

# Create the archive file (last argument is typically the output file)
# Get the last argument
for arg in "$@"; do
    LAST_ARG="$arg"
done
# Create an empty file to simulate archive creation
if [ -n "$LAST_ARG" ]; then
    touch "$LAST_ARG"
fi
exit 0
"#
    };

    // Overwrite the shimmed ar with our BusyBox-like version
    let ar_name = format!("ar{}", std::env::consts::EXE_SUFFIX);
    let ar_path = test.td.path().join(&ar_name);
    std::fs::write(&ar_path, ar_script).unwrap();

    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&ar_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ar_path, perms).unwrap();
    }

    // Create a mock ranlib that records it was called
    // Use CC_SHIM_OUT_DIR environment variable so it works across different test temp directories
    let ranlib_script = if cfg!(windows) {
        r#"@echo off
echo ranlib-called >> "%CC_SHIM_OUT_DIR%\ranlib-calls.txt"
exit /b 0
"#
    } else {
        r#"#!/bin/sh
echo "ranlib-called" >> "$CC_SHIM_OUT_DIR/ranlib-calls.txt"
exit 0
"#
    };

    let ranlib_name = format!("ranlib{}", std::env::consts::EXE_SUFFIX);
    let ranlib_path = test.td.path().join(&ranlib_name);
    std::fs::write(&ranlib_path, ranlib_script).unwrap();

    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&ranlib_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ranlib_path, perms).unwrap();
    }

    // Build with the BusyBox-like ar
    // This should succeed even though ar -s fails, because it falls back to ranlib
    test.gcc().file("foo.c").compile("foo");

    // Verify ranlib was called (fallback worked)
    let ranlib_calls = test.td.path().join("ranlib-calls.txt");
    assert!(
        ranlib_calls.exists(),
        "ranlib should have been called as fallback when ar -s failed"
    );

    // Verify the contents show ranlib was invoked
    let content = std::fs::read_to_string(&ranlib_calls).expect("Failed to read ranlib-calls.txt");
    assert!(
        content.contains("ranlib-called"),
        "ranlib-calls.txt should contain 'ranlib-called', but contains: {}",
        content
    );
}
