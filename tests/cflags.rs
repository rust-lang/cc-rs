//! Tests of the flags cc reads from `CFLAGS` and similar variables.

#![allow(deprecated)]

mod support;

use crate::support::Test;

#[test]
fn gnu_no_warnings_if_cflags() {
    let mut test = Test::gnu();
    test.env.set("CFLAGS", "-arbitrary");

    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_not_have("-Wall").must_not_have("-Wextra");
}

/// Test the ordering of flags.
///
/// 1. Default flags
/// 2. Rustflags.
/// 3. Builder flags.
/// 4. Environment flags.
#[test]
fn cflags_order() {
    let mut test = Test::gnu();

    // FIXME(madsmtm): Re-enable once `is_flag_supported` works in CI regardless of `target`.
    // test.env.set("CARGO_ENCODED_RUSTFLAGS", "-Cdwarf-version=5");

    test.env.set("CFLAGS", "-Larbitrary1");
    test.env.set("HOST_CFLAGS", "-Larbitrary2");
    test.env.set("TARGET_CFLAGS", "-Larbitrary2");
    test.env.set("CFLAGS_x86_64_unknown_none", "-Larbitrary3");
    test.env.set("CFLAGS_x86_64-unknown-none", "-Larbitrary4");

    test.gcc()
        .target("x86_64-unknown-none")
        .static_flag(true)
        .flag("-Lbuilder-flag1")
        .flag("-Lbuilder-flag2")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        // .must_have_in_order("-static", "-gdwarf-5")
        // .must_have_in_order("-gdwarf-5", "-Lbuilder-flag1")
        .must_have_in_order("-static", "-Lbuilder-flag1")
        .must_have_in_order("-Lbuilder-flag1", "-Lbuilder-flag2")
        .must_have_in_order("-Lbuilder-flag2", "-Larbitrary1")
        .must_have_in_order("-Larbitrary1", "-Larbitrary2")
        .must_have_in_order("-Larbitrary1", "-Larbitrary2")
        .must_have_in_order("-Larbitrary2", "-Larbitrary3")
        .must_have_in_order("-Larbitrary3", "-Larbitrary4");
}

#[test]
fn msvc_link_flag_in_cflags_is_ignored() {
    let mut test = Test::msvc();
    test.env.set("CFLAGS", "/W3 /link /NODEFAULTLIB");

    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("/W3")
        .must_not_have("/link")
        .must_not_have("/NODEFAULTLIB")
        .must_have("foo.c");
}

/// `x86_64` Linux gets `-march=x86-64` as a default flag, so builder and environment
/// flags can still raise the baseline.
#[test]
fn x86_64_linux_baseline_cpu_can_be_raised() {
    let mut test = Test::gnu();
    test.env.set("CFLAGS", "-march=x86-64-v3");

    test.gcc()
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .flag("-march=x86-64-v2")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have_in_order("-march=x86-64", "-march=x86-64-v2")
        .must_have_in_order("-march=x86-64-v2", "-march=x86-64-v3");
}

#[test]
fn no_default_flags_drop_x86_64_linux_baseline_cpu() {
    let target = "x86_64-unknown-linux-gnu";
    let test = Test::gnu();
    test.gcc()
        .target(target)
        .host(target)
        .no_default_flags(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_not_have("-march=x86-64");
    drop(test);

    let mut test = Test::gnu();
    test.env.set("CRATE_CC_NO_DEFAULTS", "1");
    test.gcc()
        .target(target)
        .host(target)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_not_have("-march=x86-64");
}
