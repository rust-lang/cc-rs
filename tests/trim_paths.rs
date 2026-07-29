//! Tests for inheriting path remap from cargo's unstable `-Ztrim-paths` feature.
//!
//! This test is in its own module because it modifies the environment and
//! would affect other tests when run in parallel with them.

mod support;

use crate::support::Test;
use std::env;
use std::ffi::OsString;
use std::path::Path;

const REMAP_PAIRS: &[&str] = &[
    "/path/to/pkg=foo-0.1.0",
    "/path/to/sysroot/lib/rustlib/src/rust=/rustc/1234567890abcdef",
];

const MACRO_FLAGS: &[&str] = &[
    "-fmacro-prefix-map=/path/to/pkg=foo-0.1.0",
    "-fmacro-prefix-map=/path/to/sysroot/lib/rustlib/src/rust=/rustc/1234567890abcdef",
];

const OBJECT_FLAGS: &[&str] = &[
    "-fdebug-prefix-map=/path/to/pkg=foo-0.1.0",
    "-fdebug-prefix-map=/path/to/sysroot/lib/rustlib/src/rust=/rustc/1234567890abcdef",
];

fn remap() -> OsString {
    env::join_paths(REMAP_PAIRS.iter().map(Path::new)).unwrap()
}

#[cfg(not(windows))]
#[test]
fn scope_all() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "all");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_have(flag);
    }
}

#[cfg(not(windows))]
#[test]
fn scope_macro() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "macro");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS {
        cmd.must_have(flag);
    }
    for flag in OBJECT_FLAGS {
        cmd.must_not_have(flag);
    }
}

#[cfg(not(windows))]
#[test]
fn scope_object() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "object");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in OBJECT_FLAGS {
        cmd.must_have(flag);
    }
    for flag in MACRO_FLAGS {
        cmd.must_have(flag);
    }
}

/// `diagnostics` has no C compiler equivalent; combined with `macro` only
/// the macro remap flags apply.
#[cfg(not(windows))]
#[test]
fn scope_macro_and_diagnostics() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "diagnostics,macro");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS {
        cmd.must_have(flag);
    }
    for flag in OBJECT_FLAGS {
        cmd.must_not_have(flag);
    }
}

/// `none` disables path sanitization; no remap flags should ever be emitted.
#[cfg(not(windows))]
#[test]
fn scope_none() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "none");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_not_have(flag);
    }
}

/// Without the cargo-provided env vars nothing is emitted.
#[cfg(not(windows))]
#[test]
fn no_env_vars() {
    let mut test = Test::gnu();
    test.env.remove("CARGO_TRIM_PATHS_SCOPE");
    test.env.remove("CARGO_TRIM_PATHS_REMAP");

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_not_have(flag);
    }
}

/// `Build::inherit_trim_paths(false)` opts out of the inheritance.
#[cfg(not(windows))]
#[test]
fn opt_out() {
    let mut test = Test::gnu();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "all");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc()
        .inherit_trim_paths(false)
        .file("foo.c")
        .compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_not_have(flag);
    }
}

#[test]
fn msvc_cl_scope_all() {
    let mut test = Test::msvc();
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "all");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());

    test.gcc().file("foo.c").compile("foo");

    let cmd = test.cmd(0);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_not_have(flag)
            .must_not_have(format!("/clang:{flag}"));
    }
}

#[test]
fn clang_cl_scope_all() {
    let mut test = Test::msvc();
    test.shim("clang-cl.exe");
    test.env.set("CARGO_TRIM_PATHS_SCOPE", "all");
    test.env.set("CARGO_TRIM_PATHS_REMAP", remap());
    let mut build = test.gcc();
    build
        .compiler(test.td.path().join("clang-cl.exe"))
        .file("foo.c")
        .compile("foo");

    // clang-cl forwards Clang driver options through `/clang:<arg>`:
    // https://releases.llvm.org/8.0.0/tools/clang/docs/UsersManual.html#the-clang-option
    test.cmd(0)
        .must_have("/clang:-fmacro-prefix-map=/probe=/probe");
    test.cmd(1)
        .must_have("/clang:-fdebug-prefix-map=/probe=/probe");

    let cmd = test.cmd(2);
    for flag in MACRO_FLAGS.iter().chain(OBJECT_FLAGS) {
        cmd.must_not_have(flag).must_have(format!("/clang:{flag}"));
    }
}
