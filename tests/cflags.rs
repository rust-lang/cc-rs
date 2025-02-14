//! This test is in its own module because it modifies the environment and would affect other tests
//! when run in parallel with them.
mod support;

use crate::support::Test;
use std::env;

#[test]
fn cflags() {
    gnu_no_warnings_if_cflags();
    cflags_order();
}

fn gnu_no_warnings_if_cflags() {
    env::set_var("CFLAGS", "-arbitrary");
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_not_have("-Wall").must_not_have("-Wextra");
}

/// Test the ordering of flags.
///
/// 1. Default flags
/// 2. Rustflags.
/// 3. Builder flags.
/// 4. Environment flags.
fn cflags_order() {
    // FIXME(madsmtm): Re-enable once `is_flag_supported` works in CI regardless of `target`.
    // unsafe { std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "-Cdwarf-version=5") };

    unsafe { env::set_var("CFLAGS", "-Larbitrary1") };
    unsafe { env::set_var("HOST_CFLAGS", "-Larbitrary2") };
    unsafe { env::set_var("TARGET_CFLAGS", "-Larbitrary2") };
    unsafe { env::set_var("CFLAGS_x86_64_unknown_none", "-Larbitrary3") };
    unsafe { env::set_var("CFLAGS_x86_64-unknown-none", "-Larbitrary4") };

    let test = Test::gnu();
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
