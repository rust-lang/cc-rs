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

    // pass target-cpu as -mcpu to gcc/clang
    std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "-Ctarget-cpu=neoverse-n1");
    // add the aarch64-linux-gnu-gcc shim to fake
    let test = Test::gnu();
    test.gcc()
        .target("aarch64-unknown-linux-gnu")
        .host("aarch64-unknown-linux-gnu")
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-mcpu=neoverse-n1");
}
