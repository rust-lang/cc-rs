#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::disallowed_methods)]

use std::env;
use std::process::Command;

use crate::support::Test;

mod support;

#[test]
fn gnu_smoke() {
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-O2")
        .must_have("foo.c")
        .must_not_have("-gdwarf-4")
        .must_have("-c")
        .must_have("-ffunction-sections")
        .must_have("-fdata-sections");
    test.cmd(1)
        .must_have(test.td.path().join("db3b6bfb95261072-foo.o"));
}

#[test]
fn gnu_opt_level_1() {
    let test = Test::gnu();
    test.gcc().opt_level(1).file("foo.c").compile("foo");

    test.cmd(0).must_have("-O1").must_not_have("-O2");
}

#[test]
fn gnu_opt_level_s() {
    let test = Test::gnu();
    test.gcc().opt_level_str("s").file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-Os")
        .must_not_have("-O1")
        .must_not_have("-O2")
        .must_not_have("-O3")
        .must_not_have("-Oz");
}

#[test]
fn gnu_debug() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux-none")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0)
        .must_have("-g")
        .must_not_have("-g1")
        .must_have("-gdwarf-4");
    drop(test);

    let test = Test::gnu();
    test.gcc()
        .target("x86_64-apple-darwin")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0)
        .must_have("-g")
        .must_not_have("-g1")
        .must_have("-gdwarf-2");
}

#[test]
fn gnu_debug_limited() {
    let test = Test::gnu();
    test.gcc().debug_str("limited").file("foo.c").compile("foo");
    test.cmd(0).must_not_have("-g").must_have("-g1");
}

#[test]
fn gnu_debug_none() {
    let test = Test::gnu();
    test.gcc().debug_str("none").file("foo.c").compile("foo");
    test.cmd(0).must_not_have("-g").must_not_have("-g1");
}

#[test]
fn gnu_debug_unknown() {
    let test = Test::gnu();
    test.gcc().debug_str("99").file("foo.c").compile("foo");
    test.cmd(0).must_have("-g").must_not_have("-g1");
}

#[test]
fn gnu_debug_fp_auto() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux-none")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_have("-fno-omit-frame-pointer");
    test.cmd(0).must_have("-mno-omit-leaf-frame-pointer");
}

#[test]
fn gnu_debug_fp() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux-none")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_have("-fno-omit-frame-pointer");
    test.cmd(0).must_have("-mno-omit-leaf-frame-pointer");
}

#[test]
fn gnu_debug_nofp() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux-none")
        .debug(true)
        .force_frame_pointer(false)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_not_have("-fno-omit-frame-pointer");
    test.cmd(0).must_not_have("-mno-omit-leaf-frame-pointer");
    drop(test);

    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux-none")
        .force_frame_pointer(false)
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_not_have("-fno-omit-frame-pointer");
    test.cmd(0).must_not_have("-mno-omit-leaf-frame-pointer");
}

#[test]
fn gnu_arm_neon_is_vfpv3_not_vfpv4() {
    for (target, prefix) in [
        ("thumbv7neon-unknown-linux-gnueabihf", "arm-linux-gnueabihf"),
        (
            "thumbv7neon-unknown-linux-musleabihf",
            "arm-linux-musleabihf",
        ),
        ("armv7neon-unknown-linux-gnueabihf", "arm-linux-gnueabihf"),
    ] {
        let test = Test::gnu();
        test.shim(&format!("{prefix}-gcc"))
            .shim(&format!("{prefix}-ar"));
        test.gcc().target(target).file("foo.c").compile("foo");
        test.cmd(0)
            .must_have("-mfpu=neon")
            .must_not_have("-mfpu=neon-vfpv4");
        drop(test);
    }
}

#[test]
fn gnu_warnings_into_errors() {
    let test = Test::gnu();
    test.gcc()
        .warnings_into_errors(true)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("-Werror");
}

#[test]
fn gnu_warnings() {
    let test = Test::gnu();
    test.gcc()
        .warnings(true)
        .flag("-Wno-missing-field-initializers")
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("-Wall").must_have("-Wextra");
}

#[test]
fn gnu_warnings_disabled() {
    let test = Test::gnu();
    test.gcc().warnings(false).file("foo.c").compile("foo");

    test.cmd(0).must_have("-w").must_not_have("-Wall");
}

#[test]
fn gnu_extra_warnings0() {
    let test = Test::gnu();
    test.gcc()
        .warnings(true)
        .extra_warnings(false)
        .flag("-Wno-missing-field-initializers")
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("-Wall").must_not_have("-Wextra");
}

#[test]
fn gnu_extra_warnings1() {
    let test = Test::gnu();
    test.gcc()
        .warnings(false)
        .extra_warnings(true)
        .flag("-Wno-missing-field-initializers")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have("-w")
        .must_not_have("-Wall")
        .must_have("-Wextra");
}

#[test]
fn gnu_warnings_overridable() {
    let test = Test::gnu();
    test.gcc()
        .warnings(true)
        .flag("-Wno-missing-field-initializers")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have_in_order("-Wall", "-Wno-missing-field-initializers");
}

#[test]
fn gnu_x86_64() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("x86_64-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_have("-fPIC").must_have("-m64");
    }
}

#[test]
fn gnu_x86_64_no_pic() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("x86_64-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .pic(false)
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_not_have("-fPIC");
    }
}

#[test]
fn gnu_i686() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("i686-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_have("-m32");
    }
}

#[test]
fn gnu_i686_pic() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("i686-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .pic(true)
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_have("-fPIC");
    }
}

#[test]
fn gnu_x86_64_no_plt() {
    let target = "x86_64-unknown-linux-gnu";
    let test = Test::gnu();
    test.gcc()
        .pic(true)
        .use_plt(false)
        .target(target)
        .host(target)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-fno-plt");
}

#[test]
fn gnu_aarch64_none_no_pic() {
    for target in &["aarch64-unknown-none-softfloat", "aarch64-unknown-none"] {
        let test = Test::gnu();
        test.gcc()
            .target(target)
            .host(target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_not_have("-fPIC");
    }
}

#[test]
fn gnu_uefi_no_pic() {
    for arch in &["aarch64", "i686", "x86_64"] {
        let target = format!("{}-unknown-uefi", arch);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_not_have("-fPIC");
    }
}

#[test]
fn gnu_set_stdlib() {
    let test = Test::gnu();
    test.gcc()
        .cpp_set_stdlib(Some("foo"))
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_not_have("-stdlib=foo");
}

#[test]
fn gnu_include() {
    let test = Test::gnu();
    test.gcc().include("foo/bar").file("foo.c").compile("foo");

    test.cmd(0).must_have("-I").must_have("foo/bar");
}

#[test]
fn gnu_define() {
    let test = Test::gnu();
    test.gcc()
        .define("FOO", "bar")
        .define("BAR", None)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("-DFOO=bar").must_have("-DBAR");
}

#[test]
fn gnu_compile_assembly() {
    let test = Test::gnu();
    test.gcc().file("foo.S").compile("foo");
    test.cmd(0).must_have("foo.S");
}

#[test]
fn gnu_shared() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .shared_flag(true)
        .static_flag(false)
        .compile("foo");

    test.cmd(0).must_have("-shared").must_not_have("-static");
}

#[test]
fn gnu_flag_if_supported() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .flag("-v")
        // The probe runs against the shim on this `PATH`, so tell the shim to
        // reject one flag and stand in for a compiler that does not know it.
        // Before this was hermetic the probe reached whatever compiler the
        // machine happened to have, and the test had to skip Windows.
        .env("CC_SHIM_FAIL_IF_ARG", "-Wflag-does-not-exist")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wflag-does-not-exist")
        .compile("foo");

    test.cmd(0)
        .must_have("-v")
        .must_have("-Wall")
        .must_not_have("-Wflag-does-not-exist");
}

/// `flag_if_supported` must probe even when `OUT_DIR` is unset.
/// <https://github.com/rust-lang/cc-rs/issues/1765>
#[test]
fn flag_if_supported_without_out_dir() {
    let mut test = Test::gnu();
    test.env.remove("OUT_DIR");
    test.collect_flag_supported_probes();

    let compiler = test
        .gcc_without_out_dir()
        .env("CC_SHIM_FAIL_IF_ARG", "-Wflag-does-not-exist")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wflag-does-not-exist")
        .try_get_compiler()
        .expect("try_get_compiler should succeed without OUT_DIR");

    assert!(
        compiler.args().iter().any(|a| a == "-Wall"),
        "supported flag should be applied without OUT_DIR, args: {:?}",
        compiler.args()
    );
    assert!(
        !compiler.args().iter().any(|a| a == "-Wflag-does-not-exist"),
        "unsupported flag should still be rejected without OUT_DIR, args: {:?}",
        compiler.args()
    );

    test.get_flag_supported_probes(0)
        .must_have("-Wall")
        .must_have("-c");
}

/// cc's own probing invocations run in the environment `Build::env` sets up,
/// and record only the class of probe a test asks for by name.
/// <https://github.com/rust-lang/cc-rs/issues/1859>
#[test]
fn probe_env_overrides() {
    let mut test = Test::gnu();
    test.collect_family_detection_probes()
        .collect_flag_supported_probes();
    test.gcc()
        .flag_if_supported("-Wprobed")
        .file("foo.c")
        .compile("foo");

    // Both probes went to the shim this `PATH` resolves `cc` to, rather than to
    // whatever compiler the ambient environment happens to have.
    test.get_family_detection_probes(0).must_have("-E");
    test.get_flag_supported_probes(0)
        .must_have("-Wprobed")
        .must_have("-c");

    // Neither took an `out{i}` slot, so the compile is still the first
    // invocation however many probes cc decided to run.
    test.cmd(0).must_have("foo.c").must_have("-Wprobed");
}

#[cfg(not(windows))]
#[test]
fn gnu_flag_if_supported_cpp() {
    let test = Test::gnu();
    test.gcc()
        .cpp(true)
        .file("foo.cpp")
        .flag_if_supported("-std=c++11")
        .compile("foo");

    test.cmd(0).must_have("-std=c++11");
}

#[test]
fn gnu_static() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .shared_flag(false)
        .static_flag(true)
        .compile("foo");

    test.cmd(0).must_have("-static").must_not_have("-shared");
}

#[test]
fn gnu_no_dash_dash() {
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_not_have("--");
}

#[test]
fn gnu_std_c() {
    let test = Test::gnu();
    test.gcc().file("foo.c").std("c11").compile("foo");

    test.cmd(0).must_have("-std=c11");
}

#[test]
fn msvc_smoke() {
    let test = Test::msvc();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-O2")
        .must_have("foo.c")
        .must_not_have("-Z7")
        .must_have("-c")
        .must_have("-MD")
        .must_not_have("-Tp")
        .must_not_have("-TP");
    test.cmd(1)
        .must_have(test.td.path().join("db3b6bfb95261072-foo.o"));
}

#[test]
fn msvc_opt_level_0() {
    let test = Test::msvc();
    test.gcc().opt_level(0).file("foo.c").compile("foo");

    test.cmd(0).must_not_have("-O2");
}

#[test]
fn msvc_debug() {
    let test = Test::msvc();
    test.gcc().debug(true).file("foo.c").compile("foo");
    test.cmd(0).must_have("-Z7");
}

#[test]
fn msvc_include() {
    let test = Test::msvc();
    test.gcc().include("foo/bar").file("foo.c").compile("foo");

    test.cmd(0).must_have("-I").must_have("foo/bar");
}

#[test]
fn msvc_define() {
    let test = Test::msvc();
    test.gcc()
        .define("FOO", "bar")
        .define("BAR", None)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("-DFOO=bar").must_have("-DBAR");
}

#[test]
fn msvc_link_flag_is_ignored() {
    // Regression test for issue #1331
    // https://github.com/rust-lang/cc-rs/issues/1331
    //
    // cl passes `/link` and everything after it to the linker, so the source
    // file would never reach the compiler. cc only compiles, so drop them.
    let test = Test::msvc();
    test.gcc()
        .flag("/GS-")
        .flag("/link")
        .flag("/NODEFAULTLIB")
        .define("FOO", None)
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have("/GS-")
        .must_not_have("/link")
        .must_not_have("/NODEFAULTLIB")
        .must_have("-DFOO")
        .must_have_in_order("-c", "foo.c");
}

#[test]
fn clang_cl_link_flag_is_ignored() {
    let test = Test::msvc();
    test.shim("clang-cl");
    test.gcc()
        .compiler(test.td.path().join("clang-cl"))
        .flag("-link")
        .flag("/NODEFAULTLIB")
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_not_have("-link")
        .must_not_have("/NODEFAULTLIB")
        .must_have("foo.c");
}

#[test]
fn msvc_link_flag_if_supported_is_ignored() {
    // `flag_if_supported("/link")` must not reach cl either. Each list of
    // flags is cut at its own `/link`: the entries after it in the
    // `flag_if_supported` list are dropped without being probed. `flag`
    // entries are a separate list that cc puts before the `flag_if_supported`
    // ones on the command line, so cl would never pass them to the linker, and
    // they are kept.
    let test = Test::msvc();
    let mut build = test.gcc();
    build
        .flag("/GS-")
        .flag_if_supported("/link")
        .flag_if_supported("/NODEFAULTLIB")
        .file("foo.c");
    assert!(!build.is_flag_supported("/link").unwrap());
    build.compile("foo");

    test.cmd(0)
        .must_have("/GS-")
        .must_not_have("/link")
        .must_not_have("/NODEFAULTLIB")
        .must_have_in_order("-c", "foo.c");
}

#[test]
fn gnu_link_flag_is_kept() {
    let test = Test::gnu();
    test.gcc().flag("/link").file("foo.c").compile("foo");

    test.cmd(0).must_have("/link");
}

#[test]
fn msvc_static_crt() {
    let test = Test::msvc();
    test.gcc().static_crt(true).file("foo.c").compile("foo");

    test.cmd(0).must_have("-MT");
}

#[test]
fn msvc_no_static_crt() {
    let test = Test::msvc();
    test.gcc().static_crt(false).file("foo.c").compile("foo");

    test.cmd(0).must_have("-MD");
}

#[test]
fn msvc_no_dash_dash() {
    let test = Test::msvc();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_not_have("--");
}

#[test]
fn msvc_std_c() {
    let test = Test::msvc();
    test.gcc().file("foo.c").std("c11").compile("foo");

    test.cmd(0).must_have("-std:c11");
}

#[test]
fn msvc_cpp_cc_source_not_treated_as_object() {
    // Regression test for issue #1877
    // https://github.com/rust-lang/cc-rs/issues/1877
    //
    // MSVC does not recognize `.cc` as C++ (only `.cpp` / `.cxx`). Without
    // `-Tp` it emits D9024 and treats the file as an object input, so the
    // `-Fo` output (e.g. `*-mimalloc-static.o`) is never generated.
    // libmimalloc-sys 0.1.49 writes `OUT_DIR/mimalloc-static.cc` when
    // `cpp(true)` on MSVC. Use per-file `-Tp` only for `.cc` rather than `/TP`,
    // which would compile every following input as C++.

    let test = Test::msvc();
    let src = test.td.path().join("mimalloc-static.cc");
    let intermediates = test
        .gcc()
        .cpp(true)
        .std("c++17")
        .file(&src)
        .compile_intermediates();

    assert_eq!(intermediates.len(), 1);
    assert!(
        intermediates[0]
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("mimalloc-static.o")),
        "object output should be derived from the `.cc` source, got {:?}",
        intermediates[0]
    );

    test.cmd(0)
        .must_have("-Tp")
        .must_not_have("-TP")
        .must_have("-std:c++17")
        .must_have("-c")
        .must_have(&src);

    let compile_args = &test.cmd(0).args;
    let src_pos = compile_args
        .iter()
        .position(|arg| std::path::Path::new(arg) == src.as_path())
        .expect("`.cc` source should be passed to the compiler");
    assert_eq!(
        compile_args[src_pos - 1],
        "-Tp",
        "`.cc` must be the immediate argument to `-Tp`, not a positional object: {compile_args:?}"
    );
    assert!(
        compile_args
            .iter()
            .any(|arg| arg.starts_with("-Fo") && arg.ends_with("mimalloc-static.o")),
        "object output should be the `-Fo` destination: {compile_args:?}"
    );
    assert!(
        !compile_args
            .iter()
            .any(|arg| arg.ends_with("mimalloc-static.o") && !arg.starts_with("-Fo")),
        "object output path must not appear as a source: {compile_args:?}"
    );
}

#[test]
fn msvc_cpp_does_not_pass_tp_for_c_or_cpp() {
    // `-Tp` is only required for `.cc`. Passing it (or `/TP`) for `.c` would
    // compile C as C++.
    let test = Test::msvc();
    let c_src = test.td.path().join("foo.c");
    let cpp_src = test.td.path().join("bar.cpp");
    test.gcc()
        .cpp(true)
        .file(&c_src)
        .file(&cpp_src)
        .compile("foo");

    test.cmd(0)
        .must_have(&c_src)
        .must_not_have("-Tp")
        .must_not_have("/Tp")
        .must_not_have("-TP");
    test.cmd(1)
        .must_have(&cpp_src)
        .must_not_have("-Tp")
        .must_not_have("/Tp")
        .must_not_have("-TP");
}

#[test]
fn clang_cl_cpp_cc_does_not_use_tp() {
    // clang-cl recognizes `.cc` and uses `--` as a path delimiter. `-Tp` must
    // not be passed after `--`, and is not needed before it.
    let test = Test::msvc();
    test.shim("clang-cl");
    let src = test.td.path().join("mimalloc-static.cc");
    test.gcc()
        .compiler(test.td.path().join("clang-cl"))
        .cpp(true)
        .file(&src)
        .compile_intermediates();

    test.cmd(0)
        .must_have(&src)
        .must_have("--")
        .must_not_have("-Tp")
        .must_not_have("/Tp")
        .must_not_have("-TP");
}

#[test]
fn msvc_warnings_disabled() {
    let test = Test::msvc();
    test.gcc().warnings(false).file("foo.c").compile("foo");

    test.cmd(0).must_have("-W0").must_not_have("-W4");
}

// Disable this test with the parallel feature because the execution
// order is not deterministic.
#[cfg(not(feature = "parallel"))]
#[test]
fn asm_flags() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .file("x86_64.asm")
        .file("x86_64.S")
        .asm_flag("--abc")
        .compile("foo");
    test.cmd(0).must_not_have("--abc");
    test.cmd(1).must_have("--abc");
    test.cmd(2).must_have("--abc");
}

#[test]
fn gnu_apple_sysroot() {
    let targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

    for target in targets {
        let test = Test::gnu();
        test.shim("fake-gcc")
            .gcc()
            .compiler("fake-gcc")
            .target(target)
            .host(target)
            .file("foo.c")
            .compile("foo");

        let cmd = test.cmd(0);
        cmd.must_not_have("-isysroot");
    }
}

#[test]
#[cfg(target_os = "macos")] // Invokes xcrun
fn gnu_apple_arch() {
    let cases = [
        ("x86_64-apple-darwin", "x86_64"),
        ("x86_64h-apple-darwin", "x86_64h"),
        ("aarch64-apple-darwin", "arm64"),
        ("arm64e-apple-darwin", "arm64e"),
        ("i686-apple-darwin", "i386"),
        ("aarch64-apple-ios", "arm64"),
        ("armv7s-apple-ios", "armv7s"),
        ("arm64_32-apple-watchos", "arm64_32"),
        ("armv7k-apple-watchos", "armv7k"),
    ];

    for (target, arch) in cases {
        let test = Test::gnu();
        test.shim("fake-gcc")
            .gcc()
            .compiler("fake-gcc")
            .target(target)
            .host("aarch64-apple-darwin")
            .file("foo.c")
            .compile("foo");

        let cmd = test.cmd(0);
        cmd.must_have_in_order("-arch", arch);
    }
}

#[test]
#[cfg(target_os = "macos")] // Invokes xcrun
fn gnu_apple_deployment_target() {
    let cases = [
        ("x86_64-apple-darwin", "-mmacosx-version-min=10.12"),
        ("aarch64-apple-darwin", "-mmacosx-version-min=10.12"),
        ("aarch64-apple-ios", "-miphoneos-version-min=10.0"),
        ("aarch64-apple-ios-sim", "-mios-simulator-version-min=10.0"),
        ("x86_64-apple-ios", "-mios-simulator-version-min=10.0"),
        ("aarch64-apple-ios-macabi", "-mtargetos=ios10.0-macabi"),
        ("aarch64-apple-tvos", "-mappletvos-version-min=10.0"),
        (
            "aarch64-apple-tvos-sim",
            "-mappletvsimulator-version-min=10.0",
        ),
        ("aarch64-apple-watchos", "-mwatchos-version-min=5.0"),
        (
            "aarch64-apple-watchos-sim",
            "-mwatchsimulator-version-min=5.0",
        ),
        ("aarch64-apple-visionos", "-mtargetos=xros1.0"),
        ("aarch64-apple-visionos-sim", "-mtargetos=xros1.0-simulator"),
    ];

    for (target, os_version_flag) in cases {
        let mut test = Test::gnu();

        // Avoid dependency on environment in test.
        test.env.set("MACOSX_DEPLOYMENT_TARGET", "10.12");
        test.env.set("IPHONEOS_DEPLOYMENT_TARGET", "10.0");
        test.env.set("TVOS_DEPLOYMENT_TARGET", "10.0");
        test.env.set("WATCHOS_DEPLOYMENT_TARGET", "5.0");
        test.env.set("XROS_DEPLOYMENT_TARGET", "1.0");

        test.shim("fake-gcc")
            .gcc()
            .compiler("fake-gcc")
            .target(target)
            .host("aarch64-apple-darwin")
            .file("foo.c")
            .compile("foo");

        let cmd = test.cmd(0);
        cmd.must_have(os_version_flag);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_cpp_minimums() {
    let versions = &[
        // Too low
        ("10.7", (10, 9)),
        // Minimum
        ("10.9", (10, 9)),
        // Higher
        ("11.0", (11, 0)),
    ];

    let target = "x86_64-apple-darwin";
    for (deployment_target, set_version) in versions {
        let test = Test::gnu();
        test.gcc()
            .target(target)
            .host(target)
            .cpp(true)
            .env("MACOSX_DEPLOYMENT_TARGET", deployment_target)
            .file("foo.c")
            .compile("foo");

        let exec = test.cmd(0);
        let deployment_arg = exec
            .args
            .iter()
            .find_map(|arg| arg.strip_prefix("-mmacosx-version-min="))
            .expect("no deployment target argument was set");

        let mut deployment_parts = deployment_arg.split('.').map(|v| v.parse::<u32>().unwrap());

        let major = deployment_parts.next().unwrap();
        let minor = deployment_parts.next().unwrap();

        // Check that we are on at least our minimums since this test reads from system
        // SDK state, and that can vary per-system. It should never go lower then the deployment
        // target we pass.
        assert!(major >= set_version.0);

        // If still on 10.x make sure `x` didn't go lower.
        if major == set_version.0 {
            assert!(minor >= set_version.1);
        }
    }

    let test = Test::gnu();
    test.gcc()
        .target(target)
        .host(target)
        .env("MACOSX_DEPLOYMENT_TARGET", "10.7")
        .file("foo.c")
        .compile("foo");

    // No C++ leaves it untouched
    test.cmd(0).must_have("-mmacosx-version-min=10.7");
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_tvos() {
    let target = "aarch64-apple-tvos";
    let test = Test::clang();
    test.gcc()
        .env("TVOS_DEPLOYMENT_TARGET", "9.0")
        .target(target)
        .host(target)
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("--target=arm64-apple-tvos");
    test.cmd(0).must_have("-mappletvos-version-min=9.0");
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_mac_catalyst() {
    let output = std::process::Command::new("xcrun")
        .args(["--show-sdk-path", "--sdk", "macosx"])
        .output()
        .unwrap();
    if !output.status.success() {
        return;
    }
    let sdkroot = core::str::from_utf8(&output.stdout).unwrap().trim();

    let test = Test::clang();
    test.gcc()
        .target("aarch64-apple-ios-macabi")
        .env("IPHONEOS_DEPLOYMENT_TARGET", "15.0")
        .file("foo.c")
        .compile("foo");
    let execution = test.cmd(0);

    execution.must_have("--target=arm64-apple-ios15.0-macabi");
    // --target and -mtargetos= don't mix
    execution.must_not_have("-mtargetos=");
    execution.must_have_in_order("-isysroot", sdkroot);
    execution.must_have_in_order(
        "-isystem",
        &format!("{sdkroot}/System/iOSSupport/usr/include"),
    );
    execution.must_have_in_order(
        "-iframework",
        &format!("{sdkroot}/System/iOSSupport/System/Library/Frameworks"),
    );
    execution.must_have(format!("-L{sdkroot}/System/iOSSupport/usr/lib"));
    execution.must_have(format!(
        "-F{sdkroot}/System/iOSSupport/System/Library/Frameworks"
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_tvsimulator() {
    let target = "x86_64-apple-tvos";

    let test = Test::clang();
    test.gcc()
        .env("TVOS_DEPLOYMENT_TARGET", "9.0")
        .target(target)
        .host(target)
        .file("foo.c")
        .compile("foo");

    test.cmd(0)
        .must_have("--target=x86_64-apple-tvos-simulator");
    test.cmd(0).must_have("-mappletvsimulator-version-min=9.0");
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_visionos() {
    // Only run this test if visionOS is available on the host machine
    let output = std::process::Command::new("xcrun")
        .args(["--show-sdk-version", "--sdk", "xros"])
        .output()
        .unwrap();
    if !output.status.success() {
        return;
    }

    let test = Test::clang();
    test.gcc()
        .env("XROS_DEPLOYMENT_TARGET", "1.0")
        .target("aarch64-apple-visionos")
        .file("foo.c")
        .compile("foo");

    dbg!(test.cmd(0).args);

    test.cmd(0).must_have("--target=arm64-apple-xros1.0");
    // --target and -mtargetos= don't mix.
    test.cmd(0).must_not_have("-mtargetos=");

    // Flags that don't exist.
    test.cmd(0).must_not_have("-mxros-version-min=1.0");
    test.cmd(0).must_not_have("-mxrsimulator-version-min=1.0");
}

#[cfg(target_os = "macos")]
#[test]
fn apple_sdkroot_wrong() {
    let output = std::process::Command::new("xcrun")
        .args(["--show-sdk-path", "--sdk", "iphoneos"])
        .output()
        .unwrap();
    if !output.status.success() {
        return;
    }

    let wrong_sdkroot = "/Library/Developer/CommandLineTools/SDKs/MacOSX.platform";
    let test = Test::clang();
    test.gcc()
        .env("SDKROOT", wrong_sdkroot)
        .target("aarch64-apple-ios")
        .file("foo.c")
        .compile("foo");

    dbg!(test.cmd(0).args);

    test.cmd(0)
        .must_have(core::str::from_utf8(&output.stdout).unwrap().trim());
    test.cmd(0).must_not_have(wrong_sdkroot);
}

#[test]
fn compile_intermediates() {
    let test = Test::gnu();
    let intermediates = test
        .gcc()
        .file("foo.c")
        .file("x86_64.asm")
        .file("x86_64.S")
        .asm_flag("--abc")
        .compile_intermediates();

    assert_eq!(intermediates.len(), 3);

    assert!(intermediates[0].display().to_string().contains("foo"));
    assert!(intermediates[1].display().to_string().contains("x86_64"));
    assert!(intermediates[2].display().to_string().contains("x86_64"));
}

#[test]
fn clang_android() {
    let target = "arm-linux-androideabi";

    // On Windows, we don't use the Android NDK shims for Clang, so verify that
    // we use "clang" and set the target correctly.
    #[cfg(windows)]
    {
        let mut test = Test::new();
        test.shim("clang").shim("llvm-ar");
        test.collect_ar_detection_probes();
        test.gcc()
            .target(target)
            .host("x86_64-pc-windows-msvc")
            .file("foo.c")
            .compile("foo");
        test.cmd(0).must_have("--target=arm-linux-androideabi");

        // The NDK probe runs in the environment `Build::env` set up, so it
        // finds the `llvm-ar` this test put on `PATH` rather than falling
        // back to a target-prefixed name that is not there.
        test.get_ar_detection_probes(0).must_have("--version");
    }

    // On non-Windows, we do use the shims, so make sure that we use the shim
    // and don't set the target.
    #[cfg(not(windows))]
    {
        let test = Test::new();
        test.shim("arm-linux-androideabi-clang")
            .shim("arm-linux-androideabi-ar")
            .shim("llvm-ar");
        test.gcc().target(target).file("foo.c").compile("foo");
        test.cmd(0).must_not_have("--target=arm-linux-androideabi");
    }
}

#[test]
fn parent_dir_file_path() {
    // Regression test for issue #172
    // https://github.com/rust-lang/cc-rs/issues/172
    // Ensures that files referenced with parent directory components (..)
    // have their object files placed within OUT_DIR, not in parent directories

    let test = Test::gnu();

    let intermediates = test
        .gcc()
        .file("../external_lib/test.c")
        .compile_intermediates();

    // Verify we got an object file back
    assert_eq!(
        intermediates.len(),
        1,
        "Expected exactly one intermediate object file"
    );

    let obj_path = &intermediates[0];
    let out_dir = test.td.path();

    // Verify the object file is actually within OUT_DIR
    assert!(
        obj_path.starts_with(out_dir),
        "Object file {:?} is not within OUT_DIR {:?}. This indicates the file path \
         with parent directory components (..) caused the object file to escape OUT_DIR.",
        obj_path,
        out_dir
    );
}

#[test]
fn multiple_parent_dir_references() {
    // Test deeply nested parent directory references
    // e.g., ../../deep/path/../file.c

    let test = Test::gnu();

    let intermediates = test
        .gcc()
        .file("a/b/c/../../b/c/deep.c")
        .compile_intermediates();

    assert_eq!(intermediates.len(), 1);
    let obj_path = &intermediates[0];
    let out_dir = test.td.path();

    // Must be within OUT_DIR
    assert!(
        obj_path.starts_with(out_dir),
        "Object file with multiple parent refs {:?} escaped OUT_DIR {:?}",
        obj_path,
        out_dir
    );
}

#[test]
fn parent_dir_with_multiple_files() {
    // Test that multiple files with parent directory references
    // all get properly contained in OUT_DIR

    let test = Test::gnu();

    let intermediates = test
        .gcc()
        .file("src1/../src1/file1.c")
        .file("src2/../src2/file2.c")
        .compile_intermediates();

    assert_eq!(intermediates.len(), 2, "Expected two object files");

    let out_dir = test.td.path();
    for obj_path in &intermediates {
        assert!(
            obj_path.starts_with(out_dir),
            "Object file {:?} is not within OUT_DIR {:?}",
            obj_path,
            out_dir
        );
    }
}

#[test]
fn out_dir_source_object_name_does_not_depend_on_build_path() {
    // Regression test for issue #1901
    // https://github.com/rust-lang/cc-rs/issues/1901
    // The object file name for a source under OUT_DIR must not change when
    // OUT_DIR moves, or the build path ends up in the output.

    fn object_name(mut test: Test) -> std::ffi::OsString {
        let out_dir = test.td.path().to_path_buf();
        test.env.set("OUT_DIR", &out_dir);
        let intermediates = test
            .gcc()
            .file(out_dir.join("gen.c"))
            .compile_intermediates();
        assert_eq!(intermediates.len(), 1);
        intermediates[0].file_name().unwrap().to_os_string()
    }

    // Each Test gets its own tempdir, so the two OUT_DIRs differ.
    let first = object_name(Test::gnu());
    let second = object_name(Test::gnu());
    assert_eq!(first, second, "object name depends on OUT_DIR location");
}

#[test]
fn cc_env_vars_not_overridable() {
    let test = Test::gnu();
    test.gcc()
        .env("CC_FORCE_DISABLE", "1")
        .file("foo.c")
        .compile("foo");

    // Compilation shouldn't fail here.
}

#[cfg(windows)]
#[cfg(not(disable_clang_cl_tests))]
mod msvc_clang_cl_tests {
    use super::Test;

    #[test]
    fn msvc_prefer_clang_cl_over_msvc_disabled_by_default() {
        let test = Test::msvc_autodetect();

        // When prefer_clang_cl_over_msvc is not called (default false), should use MSVC
        let compiler = test
            .gcc()
            .try_get_compiler()
            .expect("Failed to get compiler");

        // By default, should be using MSVC (cl.exe) and NOT clang-cl
        assert!(compiler.is_like_msvc(), "Should use MSVC by default");
        assert!(
            !compiler.is_like_clang_cl(),
            "Should not use clang-cl by default"
        );
    }

    #[test]
    fn msvc_prefer_clang_cl_over_msvc_enabled() {
        let test = Test::msvc_autodetect();

        let compiler = test
            .gcc()
            // When prefer_clang_cl_over_msvc is true, should use clang-cl.exe
            .prefer_clang_cl_over_msvc(true)
            .try_get_compiler()
            .expect("Failed to get compiler");

        assert!(
            compiler.is_like_clang_cl(),
            "clang-cl.exe should be identified as clang-cl-like, got {:?}",
            compiler
        );
        assert!(
            compiler.is_like_msvc(),
            "clang-cl should still be MSVC-like"
        );
    }

    #[test]
    fn msvc_prefer_clang_cl_over_msvc_respects_explicit_cc_env() {
        let mut test = Test::msvc_autodetect();

        test.env.set("CC", "cl.exe");
        let compiler = test
            .gcc()
            .prefer_clang_cl_over_msvc(true)
            .try_get_compiler()
            .expect("Failed to get compiler");

        // The preference should not override explicit compiler setting
        assert!(compiler.is_like_msvc(), "Should still be MSVC-like");
        assert!(
            !compiler.is_like_clang_cl(),
            "Should NOT use clang-cl when CC is explicitly set to cl.exe, got {:?}",
            compiler
        );
    }

    #[test]
    fn msvc_prefer_clang_cl_over_msvc_cpp_mode() {
        let test = Test::msvc_autodetect();
        let compiler = test
            .gcc()
            .cpp(true)
            .prefer_clang_cl_over_msvc(true)
            .try_get_compiler()
            .expect("Failed to get compiler");

        // Verify clang-cl.exe works correctly in C++ mode
        assert!(
            compiler.is_like_clang_cl(),
            "clang-cl.exe should be identified as clang-cl-like in C++ mode"
        );
        assert!(
            compiler.is_like_msvc(),
            "clang-cl should still be MSVC-like in C++ mode"
        );
    }
}

#[test]
fn gnu_ar_deterministic_flag() {
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(1).must_have("cqD");
    test.cmd(2).must_have("sD");
}

#[test]
fn gnu_ar_deterministic_flag_fallback() {
    let test = Test::gnu();
    test.gcc()
        .env("CC_SHIM_FAIL_IF_ARG", "cqD")
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_have("foo.c");
    test.cmd(1).must_have("cqD");
    // fallback to `ar cq` without D
    test.cmd(2).must_have("cq").must_not_have("cqD");
    test.cmd(3).must_have("s").must_not_have("sD");
}

/// Stderr from a failed `ar D` probe
/// should not be forwarded as `cargo:warning=`.
///
/// This test runs the build in a subprocess so we
/// can capture and assert on the actual stdout output.
#[test]
fn gnu_ar_probe_failure_no_warning() {
    // When invoked as subprocess, perform the build and return.
    if env::var_os("__CC_TEST_AR_PROBE_STDERR").is_some() {
        let test = Test::gnu();
        test.gcc()
            .env("CC_SHIM_FAIL_IF_ARG", "cqD")
            .file("foo.c")
            .compile("foo");
        return;
    }

    let output = Command::new(env::current_exe().unwrap())
        .env("__CC_TEST_AR_PROBE_STDERR", "1")
        .args(["--exact", "gnu_ar_probe_failure_no_warning", "--nocapture"])
        .output()
        .unwrap();
    assert!(output.status.success(), "subprocess failed: {:?}", output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The shim writes "simulated failure for arg 'cqD'" to stderr on probe failure.
    assert!(
        !stdout.contains("simulated failure"),
        "probe stderr should not appear as cargo:warning=, got:\n{}",
        stdout
    );
}
