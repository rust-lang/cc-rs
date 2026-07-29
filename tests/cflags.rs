//! This test is in its own module because it modifies the environment and would affect other tests
//! when run in parallel with them.
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
