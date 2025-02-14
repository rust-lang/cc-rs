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

/// Test the ordering of `CFLAGS*` variables.
fn cflags_order() {
    unsafe { env::set_var("CFLAGS", "-arbitrary1") };
    unsafe { env::set_var("HOST_CFLAGS", "-arbitrary2") };
    unsafe { env::set_var("TARGET_CFLAGS", "-arbitrary2") };
    unsafe { env::set_var("CFLAGS_x86_64_unknown_none", "-arbitrary3") };
    unsafe { env::set_var("CFLAGS_x86_64-unknown-none", "-arbitrary4") };
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-none")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have_in_order("-arbitrary1", "-arbitrary2")
        .must_have_in_order("-arbitrary2", "-arbitrary3")
        .must_have_in_order("-arbitrary3", "-arbitrary4");
}
