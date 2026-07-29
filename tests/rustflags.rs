#![cfg(not(windows))]
use crate::support::Test;
mod support;

#[test]
fn sanity() {
    // Sanity check - no flags
    let mut test = Test::gnu();
    test.env.set("CARGO_ENCODED_RUSTFLAGS", "");
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_not_have("-fno-omit-frame-pointer")
        .must_not_have("-mcmodel=small")
        .must_not_have("-msoft-float")
        .must_not_have("-fstack-protector-strong");
}

#[test]
fn inherits_rustflags() {
    // Correctly inherits flags from rustc
    let mut test = Test::gnu();
    test.env.set(
        "CARGO_ENCODED_RUSTFLAGS",
        "-Cforce-frame-pointers=true\u{1f}-Ccode-model=small\u{1f}-Csoft-float\u{1f}-Cdwarf-version=5\u{1f}-Zstack-protector=strong",
    );
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_have("-fno-omit-frame-pointer")
        .must_have("-mcmodel=small")
        .must_have("-msoft-float")
        .must_have("-gdwarf-5")
        .must_have("-fstack-protector-strong");
}

#[test]
fn no_stack_protector() {
    // Do *not* propagate -Zstack-protector=none
    let mut test = Test::gnu();
    test.env.set(
        "CARGO_ENCODED_RUSTFLAGS",
        "-Cforce-frame-pointers=true\u{1f}-Ccode-model=small\u{1f}-Csoft-float\u{1f}-Cdwarf-version=5\u{1f}-Zstack-protector=none",
    );
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_have("-fno-omit-frame-pointer")
        .must_have("-mcmodel=small")
        .must_have("-msoft-float")
        .must_have("-gdwarf-5")
        .must_not_have("-fno-stack-protector");
}
