//! Tests for passing the sanitizers in Cargo's `CARGO_CFG_SANITIZE` on to the
//! C compiler.

mod support;

use crate::support::Test;

const FLAG: &str = "-fsanitize=address";

/// A `Test` with `TARGET` and `CARGO_CFG_SANITIZE` set as Cargo sets them for a
/// build script.
fn sanitized(mut test: Test, target: &str, sanitizers: &str) -> Test {
    test.env.set("TARGET", target);
    test.env.set("CARGO_CFG_SANITIZE", sanitizers);
    test
}

/// A build of `foo.c` for `target`, on a host of the same target.
fn build(test: &Test, target: &str) -> cc::Build {
    let mut build = test.gcc();
    build.target(target).host(target).file("foo.c");
    build
}

fn has_any_sanitizer_flag(args: &[String]) -> bool {
    args.iter().any(|arg| arg.starts_with("-fsanitize"))
}

#[test]
fn address_gnu() {
    let target = "x86_64-unknown-linux-gnu";
    let test = sanitized(Test::gnu(), target, "address");
    build(&test, target).compile("foo");
    test.cmd(0).must_run("cc").must_have(FLAG);
}

#[test]
fn address_clang() {
    let target = "x86_64-unknown-linux-gnu";
    let mut test = sanitized(Test::clang(), target, "address");
    test.env.set("CC", "clang");
    build(&test, target).compile("foo");
    test.cmd(0).must_run("clang").must_have(FLAG);
}

#[test]
fn address_msvc() {
    let target = "x86_64-pc-windows-msvc";
    let test = sanitized(Test::msvc(), target, "address");
    build(&test, target).compile("foo");
    test.cmd(0).must_run("cl").must_have(FLAG);
}

/// The `x86_64-unknown-linux-gnuasan` target turns on `address` by default.
#[test]
fn address_gnuasan_target() {
    let target = "x86_64-unknown-linux-gnuasan";
    let test = sanitized(Test::gnu(), target, "address");
    build(&test, target)
        .host("x86_64-unknown-linux-gnu")
        .compiler(test.td.path().join("cc"))
        .compile("foo");
    test.cmd(0).must_run("cc").must_have(FLAG);
}

/// Cargo joins the sanitizers with commas. The flag is passed once, and only
/// for `address` itself.
#[test]
fn sanitizer_list() {
    let target = "x86_64-unknown-linux-gnu";
    let test = sanitized(Test::gnu(), target, "leak,address");
    build(&test, target).compile("foo");
    let cmd = test.cmd(0);
    let count = cmd.args.iter().filter(|arg| *arg == FLAG).count();
    assert_eq!(count, 1, "{:?}", cmd.args);
    cmd.must_not_have("-fsanitize=leak");
}

/// Only `address` is passed on. `hwaddress` in particular has its own
/// runtime, which objects built with `-fsanitize=address` don't work with.
#[test]
fn other_sanitizers_get_no_flag() {
    let target = "x86_64-unknown-linux-gnu";
    for sanitizers in [
        "leak",
        "memory",
        "thread",
        "hwaddress",
        "memtag",
        "cfi",
        "kcfi",
        "safestack",
        "shadow-call-stack",
        "dataflow",
        "realtime",
        "Address",
        "address ",
        "",
    ] {
        let test = sanitized(Test::gnu(), target, sanitizers);
        build(&test, target).compile("foo");
        let cmd = test.cmd(0);
        assert!(
            !has_any_sanitizer_flag(&cmd.args),
            "{sanitizers:?}: {:?}",
            cmd.args
        );
    }
}

/// GCC and Clang get the flag only on `x86_64` Linux with glibc, which CI
/// tests.
#[test]
fn untested_gnu_targets_get_no_flag() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "i686-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-unknown-linux-gnux32",
        "x86_64-pc-windows-gnu",
        // rustc reports `-Zsanitizer=kernel-address` as `address` too.
        "x86_64-unknown-none",
    ] {
        let test = sanitized(Test::gnu(), target, "address");
        build(&test, target)
            .compiler(test.td.path().join("cc"))
            .compile("foo");
        let cmd = test.cmd(0);
        cmd.must_run("cc");
        assert!(
            !has_any_sanitizer_flag(&cmd.args),
            "{target}: {:?}",
            cmd.args
        );
    }
}

/// Apple Clang's objects need Apple's runtime, not the one rustc links.
#[test]
fn apple_clang_gets_no_flag() {
    let target = "x86_64-apple-darwin";
    let mut test = sanitized(Test::clang(), target, "address");
    test.env.set("CC", "clang");
    test.env.set("MACOSX_DEPLOYMENT_TARGET", "11.0");
    build(&test, target).compile("foo");
    let cmd = test.cmd(0);
    cmd.must_run("clang");
    assert!(!has_any_sanitizer_flag(&cmd.args), "{:?}", cmd.args);
}

/// Clang targeting MSVC isn't tested either.
#[test]
fn clang_for_msvc_gets_no_flag() {
    let target = "x86_64-pc-windows-msvc";
    let mut test = sanitized(Test::clang(), target, "address");
    test.env.set("CC", "clang");
    // Only compile: there is no archiver shim for this target.
    build(&test, target).try_compile_intermediates().unwrap();
    let cmd = test.cmd(0);
    cmd.must_run("clang");
    assert!(!has_any_sanitizer_flag(&cmd.args), "{:?}", cmd.args);
}

/// `cl.exe` gets the flag only on `x86_64` Windows, and clang-cl not at all.
#[test]
fn untested_msvc_configurations_get_no_flag() {
    for target in [
        "i686-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
        "x86_64-uwp-windows-msvc",
    ] {
        let test = sanitized(Test::msvc(), target, "address");
        build(&test, target).compile("foo");
        let cmd = test.cmd(0);
        cmd.must_run("cl");
        assert!(
            !has_any_sanitizer_flag(&cmd.args),
            "{target}: {:?}",
            cmd.args
        );
    }

    let target = "x86_64-pc-windows-msvc";
    let test = sanitized(Test::msvc(), target, "address");
    test.shim("clang-cl");
    build(&test, target)
        .compiler(test.td.path().join("clang-cl"))
        .compile("foo");
    let cmd = test.cmd(0);
    cmd.must_run("clang-cl");
    assert!(!has_any_sanitizer_flag(&cmd.args), "{:?}", cmd.args);
}

/// CUDA isn't tested.
#[test]
fn cuda_gets_no_flag() {
    let target = "x86_64-unknown-linux-gnu";
    for cuda in [false, true] {
        let mut test = sanitized(Test::gnu(), target, "address");
        test.shim("nvcc");
        test.env.set("NVCC", test.td.path().join("nvcc"));
        let compiler = build(&test, target).cuda(cuda).try_get_compiler().unwrap();
        let args: Vec<String> = compiler
            .args()
            .iter()
            .map(|arg| arg.to_str().unwrap().to_owned())
            .collect();
        assert_eq!(compiler.path().file_stem().unwrap() == "nvcc", cuda);
        assert_eq!(args.iter().any(|arg| arg == FLAG), !cuda, "{args:?}");
    }
}

/// Cargo's `CARGO_CFG_*` describe its `TARGET`, not a build for another
/// target, such as a tool for the host.
#[test]
fn other_target_than_cargos_gets_no_flag() {
    let test = sanitized(Test::gnu(), "x86_64-unknown-linux-gnuasan", "address");
    build(&test, "x86_64-unknown-linux-gnu").compile("foo");
    let cmd = test.cmd(0);
    assert!(!has_any_sanitizer_flag(&cmd.args), "{:?}", cmd.args);
}

#[test]
fn no_default_flags_leave_it_out() {
    let target = "x86_64-unknown-linux-gnu";
    let test = sanitized(Test::gnu(), target, "address");
    build(&test, target).no_default_flags(true).compile("foo");
    test.cmd(0).must_not_have(FLAG);
    drop(test);

    let mut test = sanitized(Test::gnu(), target, "address");
    test.env.set("CRATE_CC_NO_DEFAULTS", "1");
    build(&test, target).compile("foo");
    test.cmd(0).must_not_have(FLAG);
}

/// The flag comes before the flags inherited from rustc, the builder's flags
/// and `CFLAGS`, so that each of them can turn it off again.
#[test]
fn flag_comes_first() {
    let target = "x86_64-unknown-linux-gnu";
    let mut test = sanitized(Test::gnu(), target, "address");
    test.env
        .set("CARGO_ENCODED_RUSTFLAGS", "-Ccode-model=small");
    test.env.set("CFLAGS", "-fno-sanitize=address");
    build(&test, target)
        .flag("-fsanitize-recover=address")
        .compile("foo");
    test.cmd(0)
        .must_have_in_order(FLAG, "-mcmodel=small")
        .must_have_in_order(FLAG, "-fsanitize-recover=address")
        .must_have_in_order(FLAG, "-fno-sanitize=address");
}
