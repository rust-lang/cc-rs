#![allow(missing_docs)]
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
        .must_not_have("-mno-omit-leaf-frame-pointer")
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
        .must_have("-mno-omit-leaf-frame-pointer")
        .must_have("-mcmodel=small")
        .must_have("-msoft-float")
        .must_have("-gdwarf-5")
        .must_have("-fstack-protector-strong");
}

#[test]
fn inherits_target_features() {
    let mut test = Test::gnu();
    test.env.set(
        "CARGO_ENCODED_RUSTFLAGS",
        "-Ctarget-feature=+ssse3,+avx\u{1f}-C\u{1f}target-feature=+avx2,-sse4a,-sse2,+crt-static,+not-a-feature",
    );
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_have("-mssse3")
        .must_have("-mavx")
        .must_have("-mavx2")
        .must_have("-mno-sse4a")
        .must_have_in_order("-mssse3", "-mno-sse4a")
        .must_not_have("-mno-sse2")
        .must_not_have("-mcrt-static")
        .must_not_have("-mnot-a-feature");
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
        .must_have("-mno-omit-leaf-frame-pointer")
        .must_have("-mcmodel=small")
        .must_have("-msoft-float")
        .must_have("-gdwarf-5")
        .must_not_have("-fno-stack-protector");
}

#[test]
fn does_not_inherit_profile_flags() {
    // Do *not* propagate `-Cprofile-generate` / `-Cprofile-use`, not even to clang: the
    // C compiler's LLVM instrprof runtime and profile format may not match rustc's.
    // https://github.com/rust-lang/cc-rs/issues/1986
    fn check(mut test: Test) {
        test.env.set(
            "CARGO_ENCODED_RUSTFLAGS",
            "-Cprofile-generate=/tmp/pgo-data\u{1f}-Cprofile-use=/tmp/merged.profdata\u{1f}-Cforce-frame-pointers=true",
        );
        test.gcc().file("foo.c").compile("foo");
        test.cmd(0)
            // Other inherited flags are unaffected.
            .must_have("-fno-omit-frame-pointer")
            .must_not_have("-fprofile-generate=/tmp/pgo-data")
            .must_not_have("-fprofile-use=/tmp/merged.profdata");
    }

    // Each `Test` holds the process environment lock, so run them one at a time.
    check(Test::gnu());

    let mut clang = Test::clang();
    clang.env.set("CC", "clang");
    check(clang);
}

#[test]
fn disabled_linker_plugin_lto_does_not_enable_clang_lto() {
    for value in ["n", "no", "off", "false"] {
        let mut test = Test::clang();
        test.env.set("CC", "clang");
        test.env.set(
            "CARGO_ENCODED_RUSTFLAGS",
            format!("-Clinker-plugin-lto={value}"),
        );
        test.gcc().file("foo.c").compile("foo");
        test.cmd_for_source("foo.c").must_not_have("-flto=thin");
    }
}

#[test]
fn enabled_linker_plugin_lto_preserves_clang_lto() {
    for flags in [
        "-Clinker-plugin-lto",
        "-Clinker-plugin-lto=yes",
        "-Clinker-plugin-lto=/path/to/LLVMgold.so",
        "-Clinker-plugin-lto=no\u{1f}-Clinker-plugin-lto",
    ] {
        let mut test = Test::clang();
        test.env.set("CC", "clang");
        test.env.set("CARGO_ENCODED_RUSTFLAGS", flags);
        test.gcc().file("foo.c").compile("foo");
        test.cmd_for_source("foo.c").must_have("-flto=thin");
    }
}
