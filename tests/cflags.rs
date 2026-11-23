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

/// `CPPFLAGS` is passed for C, C++ and preprocessed assembly, after the
/// builder's flags and before `CFLAGS` or `CXXFLAGS`, so that those can
/// override it.
#[test]
fn cppflags_before_cflags_or_cxxflags() {
    for (cpp, src, flags_var) in [(false, "foo.c", "CFLAGS"), (true, "foo.cpp", "CXXFLAGS")] {
        let mut test = Test::gnu();
        test.env.set("CPPFLAGS", "-Dfrom_cppflags -Ifrom_cppflags");
        test.env.set(flags_var, "-Dfrom_flags");

        test.gcc()
            .cpp(cpp)
            .define("from_builder", None)
            .file(src)
            .file("bar.S")
            .compile("foo");

        for src in [src, "bar.S"] {
            test.cmd_for_source(src)
                .must_have_in_order("-Dfrom_builder", "-Dfrom_cppflags")
                .must_have_in_order("-Dfrom_cppflags", "-Ifrom_cppflags")
                .must_have_in_order("-Ifrom_cppflags", "-Dfrom_flags");
        }
    }
}

/// Every form of `CPPFLAGS` that is set is used, the more specific ones last,
/// as for `CFLAGS`.
#[test]
fn cppflags_target_prefixes() {
    let mut test = Test::gnu();
    test.env.set("CPPFLAGS", "-Dplain");
    // The build cross compiles, so it reads `TARGET_CPPFLAGS`, not this.
    test.env.set("HOST_CPPFLAGS", "-Dhost");
    test.env.set("TARGET_CPPFLAGS", "-Dtarget");
    test.env
        .set("CPPFLAGS_x86_64_unknown_none", "-Dunderscores");
    test.env.set("CPPFLAGS_x86_64-unknown-none", "-Ddashes");
    test.env.set("CFLAGS", "-Dcflags");

    test.gcc()
        .target("x86_64-unknown-none")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_not_have("-Dhost")
        .must_have_in_order("-Dplain", "-Dtarget")
        .must_have_in_order("-Dtarget", "-Dunderscores")
        .must_have_in_order("-Dunderscores", "-Ddashes")
        .must_have_in_order("-Ddashes", "-Dcflags");
}

/// Unlike `CFLAGS`, `CPPFLAGS` holds no set of warning flags, so the default
/// ones stay.
#[test]
fn gnu_warnings_if_cppflags() {
    let mut test = Test::gnu();
    test.env.set("CPPFLAGS", "-Dfrom_cppflags");

    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-Dfrom_cppflags")
        .must_have("-Wall")
        .must_have("-Wextra");
}

/// `/link` and what follows it in `CPPFLAGS` are dropped like in `CFLAGS`,
/// without dropping `CFLAGS`.
#[test]
fn msvc_link_flag_in_cppflags_is_ignored() {
    let mut test = Test::msvc();
    test.env
        .set("CPPFLAGS", "/Dfrom_cppflags /link /NODEFAULTLIB");
    test.env.set("CFLAGS", "/W3");

    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have_in_order("/Dfrom_cppflags", "/W3")
        .must_not_have("/link")
        .must_not_have("/NODEFAULTLIB")
        .must_have("foo.c");
}

/// nvcc takes only its own options, so `CPPFLAGS` goes to the host compiler
/// through `-Xcompiler`, like `CXXFLAGS`.
#[test]
fn cuda_cppflags_through_xcompiler() {
    let target = "x86_64-unknown-linux-gnu";
    let mut test = Test::gnu();
    test.shim("nvcc");
    test.env.set("NVCC", test.td.path().join("nvcc"));
    test.env.set("CPPFLAGS", "-Dfrom_cppflags");

    let compiler = test
        .gcc()
        .target(target)
        .host(target)
        .cuda(true)
        .try_get_compiler()
        .unwrap();

    let args: Vec<_> = compiler
        .args()
        .iter()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert!(
        args.windows(2)
            .any(|pair| pair == ["-Xcompiler", "-Dfrom_cppflags"]),
        "{args:?}"
    );
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
