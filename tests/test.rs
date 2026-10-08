#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::disallowed_methods)]

use std::env;
use std::process::Command;

use crate::support::{GlobalEnv, Test};

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

    test.cmd_for_source(&c_src)
        .must_not_have("-Tp")
        .must_not_have("/Tp")
        .must_not_have("-TP");
    test.cmd_for_source(&cpp_src)
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

#[test]
fn asm_flags() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .file("x86_64.asm")
        .file("x86_64.S")
        .asm_flag("--abc")
        .compile("foo");
    test.cmd_for_source("foo.c").must_not_have("--abc");
    test.cmd_for_source("x86_64.asm").must_have("--abc");
    test.cmd_for_source("x86_64.S").must_have("--abc");
}

#[test]
fn msvc_masm_env_override() {
    let mut test = Test::msvc();
    test.shim("masm-wrapper");
    test.env.set("CC_MASM_ASM", "masm-wrapper --from-env");
    test.gcc().debug(true).file("foo.asm").compile("foo");

    test.cmd_for_source("foo.asm")
        .must_run("masm-wrapper")
        .must_have_in_order("--from-env", "-nologo")
        .must_have("-Zi")
        .must_not_have("-m64");
}

#[test]
fn msvc_masm_env_override_with_target_prefix() {
    let mut test = Test::msvc();
    test.shim("masm-generic").shim("masm-for-target");
    test.env.set("CC_MASM_ASM", "masm-generic --generic");
    test.env.set(
        "CC_MASM_ASM_x86_64_pc_windows_msvc",
        "masm-for-target --for-target",
    );
    test.gcc().file("foo.asm").compile("foo");

    test.cmd_for_source("foo.asm")
        .must_run("masm-for-target")
        .must_have("--for-target")
        .must_not_have("--generic");
}

#[test]
fn msvc_masm_llvm_ml_gets_target_bitness() {
    for (target, bitness, other) in [
        ("x86_64-pc-windows-msvc", "-m64", "-m32"),
        ("i686-pc-windows-msvc", "-m32", "-m64"),
    ] {
        let mut test = Test::msvc();
        test.shim("llvm-ml");
        test.env.set("CC_MASM_ASM", "llvm-ml");
        test.gcc()
            .target(target)
            .host(target)
            .debug(true)
            .file("foo.asm")
            .compile("foo");

        test.cmd_for_source("foo.asm")
            .must_run("llvm-ml")
            .must_have(bitness)
            .must_not_have(other)
            .must_not_have("-Zi");
    }
}

#[test]
fn msvc_masm_llvm_ml_name_ignores_case() {
    let mut test = Test::msvc();
    test.shim("LLVM-ML.exe");
    test.env.set("CC_MASM_ASM", "LLVM-ML.exe");
    test.gcc().debug(true).file("foo.asm").compile("foo");

    test.cmd_for_source("foo.asm")
        .must_run("LLVM-ML")
        .must_have("-m64")
        .must_not_have("-Zi");
}

#[test]
fn msvc_masm_llvm_ml_user_bitness_wins() {
    let mut test = Test::msvc();
    test.shim("llvm-ml");
    test.env.set("CC_MASM_ASM", "llvm-ml -m32");
    test.gcc().file("foo.asm").compile("foo");

    // llvm-ml takes the last `-m`.
    test.cmd_for_source("foo.asm")
        .must_run("llvm-ml")
        .must_have_in_order("-m64", "-m32");
}

// A Windows host with Visual Studio finds `ml64.exe` there, so the fallback to
// llvm-ml can only be tested elsewhere. The shims here also have no `.exe`
// suffix.
#[cfg(not(windows))]
mod msvc_masm_fallback {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use super::Test;

    /// Put clang-cl and the LLVM `tool` in a folder that is not on `PATH`, so
    /// the tool can only be found through clang-cl.
    fn clang_cl_with(test: &Test, tool: &str) -> PathBuf {
        let llvm_bin = test.td.path().join("llvm").join("bin");
        fs::create_dir_all(&llvm_bin).unwrap();
        fs::copy(&test.gcc, llvm_bin.join("clang-cl")).unwrap();
        fs::copy(&test.gcc, llvm_bin.join(tool)).unwrap();
        llvm_bin
    }

    #[test]
    fn llvm_ml_on_path() {
        let test = Test::msvc();
        test.shim("llvm-ml");
        test.gcc().debug(true).file("foo.asm").compile("foo");

        test.cmd_for_source("foo.asm")
            .must_run("llvm-ml")
            .must_have("-m64")
            .must_not_have("-Zi");
    }

    #[test]
    fn llvm_ml_next_to_clang_cl() {
        let test = Test::msvc();
        let llvm_bin = clang_cl_with(&test, "llvm-ml");
        test.gcc()
            .compiler(llvm_bin.join("clang-cl"))
            .file("foo.asm")
            .file("bar.asm")
            .compile("foo");

        for src in ["foo.asm", "bar.asm"] {
            let execution = test.cmd_for_source(src);
            execution.must_have("-m64");
            assert_eq!(execution.program, llvm_bin.join("llvm-ml"));
        }
        // One lookup serves both files.
        test.cmd_for_source("--print-search-dirs")
            .must_run("clang-cl");
    }

    #[test]
    fn ml_on_path_is_kept() {
        let test = Test::msvc();
        test.shim("ml64.exe").shim("llvm-ml");
        test.gcc().debug(true).file("foo.asm").compile("foo");

        test.cmd_for_source("foo.asm")
            .must_run("ml64")
            .must_have("-Zi")
            .must_not_have("-m64");
    }

    #[test]
    fn no_llvm_ml_for_arm() {
        let test = Test::msvc();
        test.shim("llvm-ml");
        let result = test
            .gcc()
            .target("aarch64-pc-windows-msvc")
            .host("aarch64-pc-windows-msvc")
            .cargo_warnings(false)
            .file("foo.asm")
            .try_compile("foo");

        assert!(
            !test.td.path().join("out0").exists(),
            "{:?} ran",
            test.cmd(0).program
        );
        let err = result.unwrap_err();
        assert!(err.to_string().contains("armasm64.exe"), "{err}");
    }

    #[test]
    fn clang_cl_archiver_is_llvm_lib_next_to_it() {
        let test = Test::new();
        let llvm_bin = clang_cl_with(&test, "llvm-lib");
        test.gcc()
            .target("x86_64-pc-windows-msvc")
            .host("x86_64-pc-windows-msvc")
            .compiler(llvm_bin.join("clang-cl"))
            .file("foo.c")
            .compile("foo");

        let mut out = OsString::from("-out:");
        out.push(test.td.path().join("libfoo.a"));
        assert_eq!(test.cmd_for_source(out).program, llvm_bin.join("llvm-lib"));
    }
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
    let output = GlobalEnv::output(std::process::Command::new("xcrun").args([
        "--show-sdk-path",
        "--sdk",
        "macosx",
    ]));
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
    let output = GlobalEnv::output(std::process::Command::new("xcrun").args([
        "--show-sdk-version",
        "--sdk",
        "xros",
    ]));
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
    let output = GlobalEnv::output(std::process::Command::new("xcrun").args([
        "--show-sdk-path",
        "--sdk",
        "iphoneos",
    ]));
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
fn clang_android_name_ignores_case() {
    let target = "arm-linux-androideabi";
    let name = "ARM-LINUX-ANDROIDEABI-CLANG";
    let mut test = Test::new();
    test.shim(name);
    test.env.set("CC", test.td.path().join(name));
    let host = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let compiler = test.gcc().target(target).host(host).get_compiler();
    assert!(compiler.is_like_clang());
    let target_args = compiler
        .args()
        .iter()
        .map(|arg| arg.to_str().unwrap())
        .filter(|arg| arg.starts_with("--target="))
        .collect::<Vec<_>>();

    if cfg!(windows) {
        // On Windows, cc runs the NDK's clang directly with the script's target.
        assert_eq!(compiler.path(), test.td.path().join("clang.exe"));
        assert_eq!(target_args, ["--target=arm-linux-androideabi"]);
    } else {
        // Elsewhere, cc runs the script, which passes the target itself.
        assert_eq!(compiler.path(), test.td.path().join(name));
        assert!(target_args.is_empty(), "{target_args:?}");
    }
}

// llvm-mingw's wrappers are only recognized on hosts other than Windows.
#[cfg(not(windows))]
#[test]
fn llvm_mingw_wrapper_name_ignores_case() {
    let name = "x86_64-w64-mingw32-Clang";
    let mut test = Test::new();
    test.shim(name);
    test.env.set("CC", test.td.path().join(name));
    let compiler = test
        .gcc()
        .target("x86_64-pc-windows-gnu")
        .host("x86_64-unknown-linux-gnu")
        .get_compiler();

    assert!(compiler.is_like_clang());
    assert!(
        !compiler
            .args()
            .iter()
            .any(|arg| arg.to_str().unwrap().starts_with("--target=")),
        "{:?}",
        compiler.args()
    );
}

#[test]
fn compiler_family_from_name_ignores_case() {
    // The shim can't preprocess, so cc falls back to the compiler's name.
    for (name, clang, msvc, clang_cl) in [
        ("CLANG", true, false, false),
        ("ZIG", true, false, false),
        ("Clang-CL", false, true, true),
        ("CL", false, true, false),
        ("CL.exe", false, true, false),
    ] {
        let test = Test::new();
        test.shim(name);
        let compiler = test
            .gcc()
            .target("x86_64-pc-windows-msvc")
            .host("x86_64-pc-windows-msvc")
            .compiler(test.td.path().join(name))
            .get_compiler();

        assert_eq!(
            (
                compiler.is_like_clang(),
                compiler.is_like_msvc(),
                compiler.is_like_clang_cl()
            ),
            (clang, msvc, clang_cl),
            "{name}"
        );
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

    let output = GlobalEnv::output(
        Command::new(env::current_exe().unwrap())
            .env("__CC_TEST_AR_PROBE_STDERR", "1")
            .args(["--exact", "gnu_ar_probe_failure_no_warning", "--nocapture"]),
    );
    assert!(output.status.success(), "subprocess failed: {:?}", output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The shim writes "simulated failure for arg 'cqD'" to stderr on probe failure.
    assert!(
        !stdout.contains("simulated failure"),
        "probe stderr should not appear as cargo:warning=, got:\n{}",
        stdout
    );
}

/// Each line a compiler writes to stderr becomes exactly one warning, also
/// when a failed compile's error is reported between them.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn compiler_stderr_forwarded_once_per_line() {
    // When invoked as subprocess, perform the build and return.
    if let Some(case) = env::var_os("__CC_TEST_STDERR_LINES") {
        let test = Test::gnu();
        let mut build = test.gcc();
        build
            .target("x86_64-unknown-linux-gnu")
            .host("x86_64-unknown-linux-gnu")
            .file("foo.c")
            .file("bar.c")
            .env("CC_SHIM_STDERR", "note: from the compiler");
        if case == "compile-error" {
            build.env("CC_SHIM_FAIL_IF_ARG", "-c");
            build.try_compile("foo").unwrap_err();
        } else {
            build.compile("foo");
        }
        return;
    }

    let run = |case: &str| -> Vec<String> {
        let output = GlobalEnv::output(
            Command::new(env::current_exe().unwrap())
                .env("__CC_TEST_STDERR_LINES", case)
                .env_remove("CC_ENABLE_DEBUG_OUTPUT")
                .args([
                    "--exact",
                    "compiler_stderr_forwarded_once_per_line",
                    "--nocapture",
                ]),
        );
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        let stdout: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect();
        for line in &stdout {
            assert!(!line.contains('\0'), "{stdout:#?}");
            assert!(
                !line.get(1..).unwrap_or("").contains("cargo:"),
                "{stdout:#?}"
            );
        }
        stdout
    };
    let count = |lines: &[String], line: &str| lines.iter().filter(|l| *l == line).count();

    let stdout = run("success");
    assert_eq!(
        count(&stdout, "cargo:warning=cc: note: from the compiler"),
        2,
        "{stdout:#?}"
    );

    let stdout = run("compile-error");
    assert!(
        count(&stdout, "cargo:warning=cc: simulated failure for arg '-c'") >= 1,
        "{stdout:#?}"
    );
}

/// A logger set with `message_logger` gets cc's warnings, each stderr line and
/// failed commands, while every other `cargo:` line stays on stdout.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn message_logger() {
    use std::{
        any::Any,
        io::Write,
        panic::{RefUnwindSafe, UnwindSafe},
        path::Path,
        sync::{Arc, Mutex},
    };

    use cc::{BuildMessage, BuildMessageKind, BuildMessageLogger};

    fn assert_auto_traits<T: Clone + Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}
    assert_auto_traits::<cc::Build>();

    /// Writes each message as `kind`, `extra` and the text, split by tabs.
    struct FileLogger(Mutex<std::fs::File>);

    impl BuildMessageLogger for FileLogger {
        fn log(&self, kind: BuildMessageKind, msg: BuildMessage<'_>, extra: &dyn Any) {
            let kind = match kind {
                BuildMessageKind::CommandFailed {
                    is_detection_cmd,
                    exit_status,
                    ..
                } => {
                    assert!(!exit_status.success());
                    format!(
                        "CommandFailed({:?}, {is_detection_cmd})",
                        exit_status.code()
                    )
                }
                kind => format!("{kind:?}"),
            };
            let extra = if let Some(cmd) = extra.downcast_ref::<Command>() {
                let program = Path::new(cmd.get_program()).file_stem().unwrap();
                program.to_str().unwrap().to_owned()
            } else if extra.is::<()>() {
                "()".to_owned()
            } else {
                panic!("unexpected extra for {kind}")
            };
            writeln!(self.0.lock().unwrap(), "{kind}\t{extra}\t{msg}").unwrap();
        }
    }

    // When invoked as subprocess, perform the builds and return.
    if let Some(case) = env::var_os("__CC_TEST_MESSAGE_LOGGER") {
        let log =
            std::fs::File::create(env::var_os("__CC_TEST_MESSAGE_LOGGER_FILE").unwrap()).unwrap();
        let logger = Arc::new(FileLogger(Mutex::new(log)));
        let test = if case == "msvc" {
            Test::msvc()
        } else {
            Test::gnu()
        };
        let mut build = test.gcc();
        if case != "msvc" {
            build
                .target("x86_64-unknown-linux-gnu")
                .host("x86_64-unknown-linux-gnu");
        }
        build
            .file("foo.c")
            .file("bar.c")
            // A line ending in `\r\n`, as compilers on Windows write them.
            .env("CC_SHIM_STDERR", "note: from the compiler\r")
            .message_logger(Some(logger.clone()));
        match case.to_str().unwrap() {
            "gnu" => {
                // The flag check runs in a `Build` of its own.
                build.flag_if_supported("-Wall").compile("foo");
            }
            "gnu-redirect" => {
                // The flag check's warnings stay off stdout too.
                build
                    .flag_if_supported("-Wall")
                    .cargo_warnings(false)
                    .compile("foo");
            }
            "msvc" => {
                // cc warns that `cl` can't set the C++ stdlib.
                build.cpp(true).cpp_set_stdlib("c++").compile("foo");
            }
            "compile-error" => {
                build.env("CC_SHIM_FAIL_IF_ARG", "-c");
                assert!(build.try_compile("foo").is_err());
            }
            "clone-and-remove" => {
                build.clone().compile("foo");
                build.message_logger(None).compile("bar");
            }
            "expand" => {
                // `expand` collects the compiler's output instead of streaming it.
                let mut build = test.gcc();
                build
                    .target("x86_64-unknown-linux-gnu")
                    .host("x86_64-unknown-linux-gnu")
                    .file("foo.c")
                    .message_logger(Some(logger));
                assert!(build.try_expand().is_err());
            }
            "search-dirs" => {
                // cc asks clang for its search dirs to find `llvm-ar`, and
                // falls back to `ar` when that fails.
                test.shim("clang");
                let mut build = test.gcc();
                build
                    .target("wasm32-unknown-unknown")
                    .host("x86_64-unknown-linux-gnu")
                    .env("CC_SHIM_FAIL_IF_ARG", "--print-search-dirs")
                    .message_logger(Some(logger));
                assert!(build.try_get_archiver().is_ok());
            }
            "family-detection-debug" => {
                // The shim fails the `-E` that family detection runs.
                let mut build = test.gcc();
                build
                    .target("x86_64-unknown-linux-gnu")
                    .host("x86_64-unknown-linux-gnu")
                    .cargo_debug(true)
                    .message_logger(Some(logger));
                assert!(build.try_get_compiler().is_ok());
            }
            "xcrun" => {
                // For iOS cc asks `xcrun` for the SDK version, which it can
                // do without, and for the SDK path, which it can't.
                test.shim("clang").shim("xcrun");
                for arg in ["--show-sdk-version", "--show-sdk-path"] {
                    let mut build = test.gcc();
                    build
                        .target("aarch64-apple-ios")
                        .host("x86_64-unknown-linux-gnu")
                        .env("CC_SHIM_FAIL_IF_ARG", arg)
                        .message_logger(Some(logger.clone()));
                    assert_eq!(
                        build.try_get_compiler().is_ok(),
                        arg == "--show-sdk-version"
                    );
                }
            }
            case => panic!("unknown case {case}"),
        }
        return;
    }

    let run = |case: &str| -> (Vec<String>, Vec<[String; 3]>) {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let log = dir.path().join("cc.log");
        let mut cmd = Command::new(env::current_exe().unwrap());
        cmd.env("__CC_TEST_MESSAGE_LOGGER", case)
            .env_remove("CC_ENABLE_DEBUG_OUTPUT")
            .env("__CC_TEST_MESSAGE_LOGGER_FILE", &log)
            .args(["--exact", "message_logger", "--nocapture"]);
        // The `wasm32` and iOS cases rely on cc's default compiler and
        // archiver, and the iOS case on asking `xcrun`.
        for target in ["wasm32-unknown-unknown", "aarch64-apple-ios"] {
            for var in ["CC", "AR"] {
                cmd.env_remove(format!("{var}_{target}"))
                    .env_remove(format!("{var}_{}", target.replace('-', "_")));
            }
        }
        cmd.env_remove("TARGET_CC")
            .env_remove("TARGET_AR")
            .env_remove("SDKROOT")
            .env_remove("IPHONEOS_DEPLOYMENT_TARGET");
        let output = GlobalEnv::output(&mut cmd);
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect();
        let messages = std::fs::read_to_string(&log).unwrap();
        assert!(!messages.contains('\r'), "{messages:?}");
        let messages = messages
            .lines()
            .map(|line| {
                let mut parts = line.splitn(3, '\t').map(str::to_owned);
                [(); 3].map(|()| parts.next().unwrap())
            })
            .collect();
        (stdout, messages)
    };
    let warnings = |stdout: &[String]| -> Vec<String> {
        stdout
            .iter()
            .filter_map(|line| line.strip_prefix("cargo:warning="))
            .map(str::to_owned)
            .collect()
    };
    let texts = |messages: &[[String; 3]]| -> Vec<String> {
        messages.iter().map(|[_, _, text]| text.clone()).collect()
    };
    let count = |messages: &[[String; 3]], kind: &str, extra: &str, text: &str| {
        messages
            .iter()
            .filter(|m| m[0] == kind && m[1] == extra && m[2] == text)
            .count()
    };
    let assert_link_lines = |stdout: &[String]| {
        assert!(
            stdout
                .iter()
                .any(|l| l == "cargo:rustc-link-lib=static=foo"),
            "{stdout:#?}"
        );
        assert!(
            stdout
                .iter()
                .any(|l| l.starts_with("cargo:rustc-link-search=native=")),
            "{stdout:#?}"
        );
    };
    let stderr_line = "StderrForwarding";
    let note = "cc: note: from the compiler";

    // By default the logger mirrors the warnings.
    let (stdout, messages) = run("gnu");
    assert_eq!(warnings(&stdout), texts(&messages));
    assert_eq!(
        count(&messages, stderr_line, "cc", note),
        2,
        "{messages:#?}"
    );
    assert!(
        count(&messages, stderr_line, "ar", "ar: note: from the compiler") >= 1,
        "{messages:#?}"
    );
    assert!(
        messages
            .iter()
            .all(|m| m[0] == stderr_line || (m[0] == "GeneralWarning" && m[1] == "()")),
        "{messages:#?}"
    );
    assert_link_lines(&stdout);

    // With cargo warnings off, the messages only go to the logger.
    let (stdout, messages) = run("gnu-redirect");
    assert_eq!(warnings(&stdout), Vec::<String>::new());
    assert_eq!(
        count(&messages, stderr_line, "cc", note),
        2,
        "{messages:#?}"
    );
    assert_link_lines(&stdout);

    // cc's own warnings are general warnings.
    let (stdout, messages) = run("msvc");
    assert_eq!(warnings(&stdout), texts(&messages));
    assert!(
        messages
            .iter()
            .any(|[kind, extra, text]| kind == "GeneralWarning"
                && extra == "()"
                && text.contains("cpp_set_stdlib is specified")),
        "{messages:#?}"
    );
    assert_link_lines(&stdout);

    // A failed compile's diagnostics come before the failed command.
    let (_, messages) = run("compile-error");
    let failed = messages
        .iter()
        .position(|[kind, extra, text]| {
            kind == "CommandFailed(Some(1), false)"
                && extra == "cc"
                && text.starts_with("command did not execute successfully")
        })
        .unwrap_or_else(|| panic!("{messages:#?}"));
    assert!(
        count(
            &messages[..failed],
            stderr_line,
            "cc",
            "cc: simulated failure for arg '-c'"
        ) >= 1,
        "{messages:#?}"
    );
    // With `parallel`, the error isn't passed a second time as a warning.
    assert!(
        messages
            .iter()
            .all(|m| m[0] != "GeneralWarning" || !m[2].contains(&messages[failed][2])),
        "{messages:#?}"
    );

    // Output that cc collects reaches the logger too.
    let (_, messages) = run("expand");
    let failed = messages
        .iter()
        .position(|[kind, extra, _]| kind == "CommandFailed(Some(1), false)" && extra == "cc")
        .unwrap_or_else(|| panic!("{messages:#?}"));
    assert_eq!(
        count(
            &messages[..failed],
            stderr_line,
            "cc",
            "cc: the shim cannot preprocess"
        ),
        1,
        "{messages:#?}"
    );

    // A failed detection command is still reported, flagged as one.
    let (_, messages) = run("search-dirs");
    let failed: Vec<_> = messages
        .iter()
        .filter(|[kind, _, _]| kind.starts_with("CommandFailed"))
        .collect();
    assert_eq!(failed.len(), 1, "{messages:#?}");
    let [kind, extra, text] = failed[0];
    assert_eq!(
        [kind.as_str(), extra.as_str()],
        ["CommandFailed(Some(1), true)", "clang"],
        "{messages:#?}"
    );
    assert!(text.contains("--print-search-dirs"), "{messages:#?}");
    assert_eq!(
        count(
            &messages,
            stderr_line,
            "clang",
            "clang: simulated failure for arg '--print-search-dirs'"
        ),
        1,
        "{messages:#?}"
    );

    // Family detection reports its failures only with `cargo_debug`, and the
    // shim makes its `-E` fail.
    let (_, messages) = run("family-detection-debug");
    let failed: Vec<_> = messages
        .iter()
        .filter(|[kind, _, _]| kind.starts_with("CommandFailed"))
        .collect();
    assert_eq!(failed.len(), 1, "{messages:#?}");
    let [kind, extra, text] = failed[0];
    assert_eq!(
        [kind.as_str(), extra.as_str()],
        ["CommandFailed(Some(1), true)", "cc"],
        "{messages:#?}"
    );
    assert!(text.contains("\"-E\""), "{messages:#?}");

    // cc goes on without the SDK version, but not without the SDK path.
    let (_, messages) = run("xcrun");
    let failed: Vec<_> = messages
        .iter()
        .filter(|[kind, _, _]| kind.starts_with("CommandFailed"))
        .map(|[kind, extra, text]| {
            let arg = ["--show-sdk-version", "--show-sdk-path"]
                .into_iter()
                .find(|arg| text.contains(arg));
            (kind.as_str(), extra.as_str(), arg)
        })
        .collect();
    assert_eq!(
        failed,
        [
            (
                "CommandFailed(Some(1), true)",
                "xcrun",
                Some("--show-sdk-version")
            ),
            (
                "CommandFailed(Some(1), false)",
                "xcrun",
                Some("--show-sdk-path")
            ),
        ],
        "{messages:#?}"
    );

    // Clones share the logger, and `None` removes it.
    let (stdout, messages) = run("clone-and-remove");
    assert_eq!(
        count(&messages, stderr_line, "cc", note),
        2,
        "{messages:#?}"
    );
    assert_eq!(
        warnings(&stdout).iter().filter(|w| *w == note).count(),
        4,
        "{stdout:#?}"
    );
}

/// With `cpp_link_stdlib_static`, the C++ stdlib is emitted with `-bundle` once
/// per `Build`, `wasm32`, `pauthtest` and Apple keep a plain `static=`, and a
/// stdlib value that already names a link kind is emitted unchanged.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn cpp_link_stdlib_static_metadata() {
    // When invoked as subprocess, perform the builds and return.
    if let Some(case) = env::var_os("__CC_TEST_CPP_LINK_STDLIB_STATIC") {
        let test = Test::gnu();
        test.shim("clang++");
        let mut build = test.gcc();
        build
            .target("x86_64-unknown-linux-gnu")
            .host("x86_64-unknown-linux-gnu")
            .cpp(true)
            .cpp_link_stdlib_static(true)
            .file("foo.cpp")
            .archiver(test.td.path().join("ar"));
        match case.to_str().unwrap() {
            "one-build" => {
                build.compile("foo");
                build.compile("bar");
                build.clone().compile("baz");
            }
            "two-builds" => {
                build.clone().compile("foo");
                let mut other = test.gcc();
                other
                    .target("x86_64-unknown-linux-gnu")
                    .host("x86_64-unknown-linux-gnu")
                    .cpp(true)
                    .cpp_link_stdlib_static(true)
                    .file("foo.cpp")
                    .archiver(test.td.path().join("ar"))
                    .compile("bar");
            }
            "metadata-off-first" => {
                build.cargo_metadata(false).compile("foo");
                build.cargo_metadata(true).compile("bar");
            }
            target @ ("wasm32-unknown-unknown" | "aarch64-unknown-linux-pauthtest") => {
                build
                    .target(target)
                    .compiler(test.td.path().join("clang++"));
                build.compile("foo");
                build.compile("bar");
            }
            "x86_64-apple-darwin" => {
                build.target("x86_64-apple-darwin");
                build.compile("foo");
                build.compile("bar");
            }
            case => panic!("unknown case {case}"),
        }
        return;
    }

    let link_lib_lines = |case: &str, envs: &[(&str, &str)]| -> Vec<String> {
        let output = GlobalEnv::output(
            Command::new(env::current_exe().unwrap())
                .env("__CC_TEST_CPP_LINK_STDLIB_STATIC", case)
                .env_remove("CXXSTDLIB")
                .envs(envs.iter().copied())
                .args(["--exact", "cpp_link_stdlib_static_metadata", "--nocapture"]),
        );
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("cargo:rustc-link-lib="))
            .map(str::to_owned)
            .collect()
    };

    // Several compiles on one `Build` and its clones emit the stdlib once.
    assert_eq!(
        link_lib_lines("one-build", &[]),
        [
            "static=foo",
            "static:-bundle=stdc++",
            "static=bar",
            "static=baz"
        ]
    );
    // The state is per `Build`, not global.
    assert_eq!(
        link_lib_lines("two-builds", &[]),
        [
            "static=foo",
            "static:-bundle=stdc++",
            "static=bar",
            "static:-bundle=stdc++"
        ]
    );
    // A compile that emits no metadata doesn't use up the one emission.
    assert_eq!(
        link_lib_lines("metadata-off-first", &[]),
        ["static=bar", "static:-bundle=stdc++"]
    );
    // `wasm32` and `pauthtest` may also link the C++ stdlib through
    // `rustc-flags`, so they keep `static=`.
    assert_eq!(
        link_lib_lines("wasm32-unknown-unknown", &[]),
        ["static=foo", "static=stdc++", "static=bar", "static=stdc++"]
    );
    assert_eq!(
        link_lib_lines(
            "aarch64-unknown-linux-pauthtest",
            &[
                ("PAUTHTEST_SYSROOT", "sysroot"),
                ("PAUTHTEST_RESOURCE_DIR", "resource-dir")
            ]
        ),
        ["static=foo", "static=c++", "static=bar", "static=c++"]
    );
    // On Apple targets rustc passes no static hint and ld64 prefers the dylib,
    // so the stdlib keeps `static=` there too.
    assert_eq!(
        link_lib_lines("x86_64-apple-darwin", &[]),
        ["static=foo", "static=c++", "static=bar", "static=c++"]
    );
    // A value that already names a link kind is emitted unchanged, once per `Build`.
    assert_eq!(
        link_lib_lines("one-build", &[("CXXSTDLIB", "static:-bundle=c++")]),
        [
            "static=foo",
            "static:-bundle=c++",
            "static=bar",
            "static=baz"
        ]
    );
    assert_eq!(
        link_lib_lines("one-build", &[("CXXSTDLIB", "dylib=stdc++")]),
        ["static=foo", "dylib=stdc++", "static=bar", "static=baz"]
    );
}

/// The `CXXSTDLIB_STATIC` environment variable links the C++ stdlib statically
/// for a `Build` that doesn't call `cpp_link_stdlib_static`, keeping its stdlib
/// name.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn cxxstdlib_static_env_metadata() {
    // When invoked as subprocess, perform the builds and return.
    if let Some(case) = env::var_os("__CC_TEST_CXXSTDLIB_STATIC") {
        let test = Test::gnu();
        let mut build = test.gcc();
        build
            .target("x86_64-unknown-linux-gnu")
            .host("x86_64-unknown-linux-gnu")
            .cpp(true)
            .cpp_link_stdlib("stdc++")
            .file("foo.cpp")
            .archiver(test.td.path().join("ar"));
        match case.to_str().unwrap() {
            "stdlib" => {}
            "no-stdlib" => {
                build.cpp_link_stdlib(None);
            }
            "static-false" => {
                build.cpp_link_stdlib_static(false);
            }
            "x86_64-apple-darwin" => {
                build.target("x86_64-apple-darwin").cpp_link_stdlib("c++");
            }
            case => panic!("unknown case {case}"),
        }
        build.compile("foo");
        build.compile("bar");
        return;
    }

    let metadata_lines = |case: &str, envs: &[(&str, &str)]| -> Vec<String> {
        let output = GlobalEnv::output(
            Command::new(env::current_exe().unwrap())
                .env("__CC_TEST_CXXSTDLIB_STATIC", case)
                .env_remove("CXXSTDLIB")
                .env_remove("CXXSTDLIB_STATIC")
                .env_remove("HOST_CXXSTDLIB_STATIC")
                .env_remove("TARGET_CXXSTDLIB_STATIC")
                .env_remove("CXXSTDLIB_STATIC_x86_64-unknown-linux-gnu")
                .env_remove("CXXSTDLIB_STATIC_x86_64_unknown_linux_gnu")
                .env_remove("CXXSTDLIB_STATIC_x86_64-apple-darwin")
                .env_remove("CXXSTDLIB_STATIC_x86_64_apple_darwin")
                .envs(envs.iter().copied())
                .args(["--exact", "cxxstdlib_static_env_metadata", "--nocapture"]),
        );
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| {
                line.starts_with("cargo:rustc-link-lib=")
                    || (line.starts_with("cargo:rerun-if-env-changed=")
                        && line.contains("CXXSTDLIB_STATIC"))
            })
            .map(str::to_owned)
            .collect()
    };
    let link_lib_lines = |case: &str, envs: &[(&str, &str)]| -> Vec<String> {
        metadata_lines(case, envs)
            .iter()
            .filter_map(|line| line.strip_prefix("cargo:rustc-link-lib="))
            .map(str::to_owned)
            .collect()
    };

    // Unset or false, nothing changes.
    let dynamic = ["static=foo", "stdc++", "static=bar", "stdc++"];
    assert_eq!(link_lib_lines("stdlib", &[]), dynamic);
    for value in ["", "0", "no", "false"] {
        assert_eq!(
            link_lib_lines("stdlib", &[("CXXSTDLIB_STATIC", value)]),
            dynamic
        );
    }
    // Set, the crate's stdlib is linked statically, once per `Build`.
    let static_once = ["static=foo", "static:-bundle=stdc++", "static=bar"];
    for (key, value) in [
        ("CXXSTDLIB_STATIC", "1"),
        ("CXXSTDLIB_STATIC", "true"),
        ("HOST_CXXSTDLIB_STATIC", "1"),
        ("CXXSTDLIB_STATIC_x86_64-unknown-linux-gnu", "1"),
        ("CXXSTDLIB_STATIC_x86_64_unknown_linux_gnu", "1"),
    ] {
        assert_eq!(link_lib_lines("stdlib", &[(key, value)]), static_once);
    }
    // It also overrides an explicit `cpp_link_stdlib_static(false)`.
    assert_eq!(
        link_lib_lines("static-false", &[("CXXSTDLIB_STATIC", "1")]),
        static_once
    );
    // A target specific value takes precedence over the plain one.
    assert_eq!(
        link_lib_lines(
            "stdlib",
            &[
                ("CXXSTDLIB_STATIC", "1"),
                ("CXXSTDLIB_STATIC_x86_64_unknown_linux_gnu", "0")
            ]
        ),
        dynamic
    );
    // No stdlib stays no stdlib.
    assert_eq!(
        link_lib_lines("no-stdlib", &[("CXXSTDLIB_STATIC", "1")]),
        ["static=foo", "static=bar"]
    );
    // Apple keeps `static=`, as with `cpp_link_stdlib_static(true)`.
    assert_eq!(
        link_lib_lines("x86_64-apple-darwin", &[("CXXSTDLIB_STATIC", "1")]),
        ["static=foo", "static=c++", "static=bar", "static=c++"]
    );
    // Cross compiling, the `TARGET_` prefixed form is read.
    assert_eq!(
        link_lib_lines("x86_64-apple-darwin", &[("TARGET_CXXSTDLIB_STATIC", "1")]),
        ["static=foo", "static=c++", "static=bar", "static=c++"]
    );
    // Cargo reruns the build script when the variable changes.
    let lines = metadata_lines("stdlib", &[]);
    for key in [
        "CXXSTDLIB_STATIC_x86_64-unknown-linux-gnu",
        "CXXSTDLIB_STATIC_x86_64_unknown_linux_gnu",
        "HOST_CXXSTDLIB_STATIC",
        "CXXSTDLIB_STATIC",
    ] {
        assert!(
            lines.contains(&format!("cargo:rerun-if-env-changed={key}")),
            "missing rerun-if-env-changed for {key}: {lines:?}"
        );
    }
}

/// What the C++ stdlib probe of a Clang that uses libc++ prints.
const LIBCXX_PROBE_STDOUT: &str = "# 1 \"detect_cpp_stdlib.cpp\"\n#pragma message(\"libcxx\")\n";

/// A `Build` for a C++ library with Clang, on a target that links libstdc++ by
/// default, whose C++ stdlib probe prints `probe_stdout`, or fails without it.
fn clang_cpp_build(test: &Test, probe_stdout: Option<&str>) -> cc::Build {
    let mut build = test.gcc();
    build
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .cpp(true)
        .file("foo.cpp")
        .compiler(test.td.path().join("clang++"))
        .archiver(test.td.path().join("ar"));
    if let Some(stdout) = probe_stdout {
        build.env("CC_SHIM_STDOUT_FOR_CPP_STDLIB_DETECTION", stdout);
    }
    build
}

/// Like [`clang_cpp_build`], with a probe that finds libc++.
fn libcxx_clang_build(test: &Test) -> cc::Build {
    clang_cpp_build(test, Some(LIBCXX_PROBE_STDOUT))
}

/// On a target that links libstdc++ by default, a Clang that uses libc++ gets
/// `c++` linked instead, and anything else keeps `stdc++`.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn cpp_stdlib_detection_metadata() {
    // When invoked as subprocess, perform the builds and return.
    if let Some(case) = env::var_os("__CC_TEST_CPP_STDLIB_DETECTION") {
        let mut test = Test::clang();
        test.shim("c++").shim("nvcc");
        let library = test.td.path().join("libfoo.a");
        let mut build = libcxx_clang_build(&test);
        match case.to_str().unwrap() {
            "libcxx" => {}
            "libstdcxx" => {
                // Only the message counts, not a path that mentions libcxx.
                let probe_stdout = "# 1 \"/build/libcxx-sys/out/detect_cpp_stdlib.cpp\"\n";
                build = clang_cpp_build(&test, Some(probe_stdout));
            }
            "probe-fails" => {
                build = clang_cpp_build(&test, None);
            }
            "gcc" => {
                build.compiler(test.td.path().join("c++"));
            }
            "cpp-link-stdlib" => {
                build.cpp_link_stdlib("stdc++");
            }
            "nostdinc++" => {
                build
                    .flag("-nostdinc++")
                    .flag("-isystemvendor/libc++/include");
            }
            "wasm32" => {
                build.target("wasm32-unknown-unknown");
            }
            "windows-gnu" => {
                // A Windows compiler ends its lines with `\r\n`.
                let probe_stdout = LIBCXX_PROBE_STDOUT.replace('\n', "\r\n");
                build = clang_cpp_build(&test, Some(&probe_stdout));
                build
                    .target("x86_64-pc-windows-gnu")
                    .host("x86_64-pc-windows-gnu");
                cc::emit_link_directives(&build, &library);
                cc::emit_link_directives(&build, &library);
                return;
            }
            "cuda" => {
                test.env.set("CXX", test.td.path().join("clang++"));
                test.env.set("NVCC", test.td.path().join("nvcc"));
                let mut build = test.gcc();
                build
                    .target("x86_64-unknown-linux-gnu")
                    .host("x86_64-unknown-linux-gnu")
                    .cuda(true)
                    .cudart("none")
                    .env(
                        "CC_SHIM_STDOUT_FOR_CPP_STDLIB_DETECTION",
                        LIBCXX_PROBE_STDOUT,
                    );
                cc::emit_link_directives(&build, &library);
                cc::emit_link_directives(&build, &library);
                return;
            }
            "no-out-dir" => {
                let mut build = test.gcc_without_out_dir();
                build
                    .target("x86_64-unknown-linux-gnu")
                    .host("x86_64-unknown-linux-gnu")
                    .cpp(true)
                    .compiler(test.td.path().join("clang++"))
                    .env(
                        "CC_SHIM_STDOUT_FOR_CPP_STDLIB_DETECTION",
                        LIBCXX_PROBE_STDOUT,
                    );
                cc::emit_link_directives(&build, &library);
                cc::emit_link_directives(&build.clone(), &library);
                return;
            }
            "out-dir-fails-once" => {
                // A probe that fails for a reason outside the compiler is not
                // remembered, so the next one can still find libc++.
                let blocker = test.td.path().join("blocker");
                std::fs::write(&blocker, "").unwrap();
                build.out_dir(blocker.join("out"));
                cc::emit_link_directives(&build, &library);
                std::fs::remove_file(&blocker).unwrap();
                cc::emit_link_directives(&build, &library);
                return;
            }
            "force-disable" => {
                cc::emit_link_directives(&build, &library);
                return;
            }
            case => panic!("unknown case {case}"),
        }
        build.compile("foo");
        build.compile("bar");
        return;
    }

    let cargo_lines = |case: &str, envs: &[(&str, &str)]| -> Vec<String> {
        let output = GlobalEnv::output(
            Command::new(env::current_exe().unwrap())
                .env("__CC_TEST_CPP_STDLIB_DETECTION", case)
                .env_remove("CXXSTDLIB")
                .env_remove("CXXSTDLIB_STATIC")
                .env_remove("CC_FORCE_DISABLE")
                .env_remove("CC_ENABLE_DEBUG_OUTPUT")
                .env_remove("OUT_DIR")
                .envs(envs.iter().copied())
                .args(["--exact", "cpp_stdlib_detection_metadata", "--nocapture"]),
        );
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.find("cargo:").map(|i| line[i..].to_owned()))
            .collect()
    };
    let link_lib_lines = |case: &str, envs: &[(&str, &str)]| -> Vec<String> {
        cargo_lines(case, envs)
            .iter()
            .filter_map(|line| line.strip_prefix("cargo:rustc-link-lib="))
            .map(str::to_owned)
            .collect()
    };
    // The probe prints nothing itself, and works out the compiler again
    // without repeating the lines and warnings the build already printed.
    let other_lines = |case: &str| -> Vec<String> {
        cargo_lines(case, &[])
            .into_iter()
            .filter(|line| {
                !line.starts_with("cargo:rustc-link-")
                    && !(line.starts_with("cargo:rerun-if-env-changed=")
                        && line.contains("CXXSTDLIB"))
            })
            // Leave out the paths, which differ between runs.
            .map(|line| line.split('"').next().unwrap().to_owned())
            .collect()
    };
    let without_probe = other_lines("cpp-link-stdlib");
    assert!(
        without_probe
            .iter()
            .any(|line| line.starts_with("cargo:warning=")),
        "{without_probe:?}"
    );
    assert_eq!(other_lines("libcxx"), without_probe);
    assert_eq!(other_lines("probe-fails"), without_probe);

    let libcxx = ["static=foo", "c++", "static=bar", "c++"];
    let libstdcxx = ["static=foo", "stdc++", "static=bar", "stdc++"];
    assert_eq!(link_lib_lines("libcxx", &[]), libcxx);
    // The static stdlib follows the detected one.
    assert_eq!(
        link_lib_lines("libcxx", &[("CXXSTDLIB_STATIC", "1")]),
        ["static=foo", "static:-bundle=c++", "static=bar"]
    );
    assert_eq!(
        link_lib_lines("windows-gnu", &[]),
        ["static=foo", "c++", "static=foo", "c++"]
    );
    assert_eq!(
        link_lib_lines("no-out-dir", &[]),
        ["static=foo", "c++", "static=foo", "c++"]
    );
    assert_eq!(
        link_lib_lines("out-dir-fails-once", &[]),
        ["static=foo", "stdc++", "static=foo", "c++"]
    );

    // Everything else keeps `stdc++`.
    assert_eq!(link_lib_lines("libstdcxx", &[]), libstdcxx);
    assert_eq!(link_lib_lines("probe-fails", &[]), libstdcxx);
    assert_eq!(link_lib_lines("gcc", &[]), libstdcxx);
    assert_eq!(link_lib_lines("cpp-link-stdlib", &[]), libstdcxx);
    // A build that compiles against C++ headers of its own, such as a vendored
    // libc++ it links in some other way, keeps `stdc++` too.
    assert_eq!(link_lib_lines("nostdinc++", &[]), libstdcxx);
    assert_eq!(
        link_lib_lines("libcxx", &[("CXXSTDLIB", "stdc++")]),
        libstdcxx
    );
    assert_eq!(
        link_lib_lines("wasm32", &[]),
        ["static=foo", "stdc++", "static=bar", "stdc++"]
    );
    assert_eq!(
        link_lib_lines("cuda", &[]),
        ["static=foo", "stdc++", "static=foo", "stdc++"]
    );
    assert_eq!(
        link_lib_lines("force-disable", &[("CC_FORCE_DISABLE", "1")]),
        ["static=foo", "stdc++"]
    );
}

/// The C++ stdlib probe runs the compiler with the flags the build compiles
/// with, but without the ones that write a dependency file, so it can't write
/// one next to the build's own or overwrite it.
#[test]
fn cpp_stdlib_probe_drops_dependency_output_flags() {
    let mut test = Test::clang();
    test.collect_cpp_stdlib_probes();
    test.env.set("CXXFLAGS", "-DFROM_ENV -MP -MFenv.d");
    let mut build = libcxx_clang_build(&test);
    for flag in [
        "-stdlib=libc++",
        "-MD",
        "-MF",
        "deps.d",
        "-MMD",
        "-MTtarget",
        "-MQ",
        "quoted",
        "-MJ",
        "db.json",
        "-Wp,-MD,wp.d",
    ] {
        build.flag(flag);
    }
    build.compile("foo");

    let probe = test
        .get_cpp_stdlib_probes(0)
        .expect("no C++ stdlib probe ran");
    probe
        .must_have("-E")
        .must_have("-stdlib=libc++")
        .must_have("-DFROM_ENV");
    for arg in [
        "-MD",
        "-MF",
        "deps.d",
        "-MMD",
        "-MTtarget",
        "-MQ",
        "quoted",
        "-MJ",
        "db.json",
        "-Wp,-MD,wp.d",
        "-MP",
        "-MFenv.d",
    ] {
        probe.must_not_have(arg);
    }
    // The compile itself keeps them.
    test.cmd_for_source("foo.cpp")
        .must_have("-MD")
        .must_have("deps.d")
        .must_have("-MFenv.d");
}

/// The C++ stdlib probe runs once per command, shared by clones, and again for
/// a command or environment it hasn't seen.
#[test]
fn cpp_stdlib_probe_is_cached_per_command() {
    let mut test = Test::clang();
    test.collect_cpp_stdlib_probes();
    let build = libcxx_clang_build(&test);
    build.compile("foo");
    build.compile("bar");
    build.clone().compile("baz");
    assert!(test.get_cpp_stdlib_probes(0).is_some(), "no probe ran");
    assert!(test.get_cpp_stdlib_probes(1).is_none(), "probed again");

    build.clone().flag("-DOTHER").compile("foo");
    test.get_cpp_stdlib_probes(1)
        .expect("a new flag did not probe again")
        .must_have("-DOTHER");

    build.clone().env("CC_TEST_OTHER_ENV", "1").compile("foo");
    assert!(
        test.get_cpp_stdlib_probes(2).is_some(),
        "a new environment did not probe again"
    );
    assert!(test.get_cpp_stdlib_probes(3).is_none());
}

/// Guard test: the C++ stdlib probe doesn't run where its answer isn't used.
#[test]
fn cpp_stdlib_probe_skipped() {
    let library = |test: &Test| test.td.path().join("libfoo.a");
    let check = |configure: &dyn Fn(&Test, &mut cc::Build)| {
        let mut test = Test::clang();
        test.collect_cpp_stdlib_probes().shim("c++");
        let mut build = libcxx_clang_build(&test);
        configure(&test, &mut build);
        cc::emit_link_directives(&build, library(&test));
        assert!(test.get_cpp_stdlib_probes(0).is_none(), "the probe ran");
    };

    // The stdlib is set explicitly.
    check(&|_, build| {
        build.cpp_link_stdlib("stdc++");
    });
    check(&|_, build| {
        build.cpp_link_stdlib(None);
    });
    check(&|_, build| {
        build.cpp_set_stdlib("c++");
    });
    // Nothing is linked without metadata.
    check(&|_, build| {
        build.cargo_metadata(false);
    });
    // Only Clang is probed.
    check(&|test, build| {
        build.compiler(test.td.path().join("c++"));
    });
    // The build picks its own C++ headers.
    check(&|_, build| {
        build.flag("-nostdinc++");
    });
    check(&|_, build| {
        build.flag("-nostdinc");
    });
    // Targets that don't link libstdc++ by default.
    check(&|_, build| {
        build.target("aarch64-linux-android");
    });
    check(&|_, build| {
        build.target("x86_64-unknown-freebsd");
    });
    check(&|_, build| {
        build.target("wasm32-unknown-unknown");
    });

    let mut test = Test::clang();
    test.collect_cpp_stdlib_probes();
    test.env.set("CXXSTDLIB", "stdc++");
    cc::emit_link_directives(&libcxx_clang_build(&test), library(&test));
    assert!(test.get_cpp_stdlib_probes(0).is_none(), "the probe ran");
    drop(test);

    let mut test = Test::clang();
    test.collect_cpp_stdlib_probes();
    test.env.set("CC_FORCE_DISABLE", "1");
    cc::emit_link_directives(&libcxx_clang_build(&test), library(&test));
    assert!(test.get_cpp_stdlib_probes(0).is_none(), "the probe ran");
}

/// Guard test: like family detection, the C++ stdlib probe only sends its
/// stderr and its failure to the build's logger with debug output.
#[test]
fn cpp_stdlib_probe_logs_nothing() {
    use cc::{BuildMessage, BuildMessageKind, BuildMessageLogger};
    use std::{
        any::Any,
        sync::{Arc, Mutex},
    };

    struct Messages(Mutex<Vec<String>>);

    impl BuildMessageLogger for Messages {
        fn log(&self, _: BuildMessageKind, msg: BuildMessage<'_>, _: &dyn Any) {
            self.0.lock().unwrap().push(msg.to_string());
        }
    }

    let mut test = Test::clang();
    test.collect_cpp_stdlib_probes();
    let messages = Arc::new(Messages(Mutex::new(Vec::new())));
    let mut build = clang_cpp_build(&test, None);
    build
        .cargo_debug(false)
        .cargo_warnings(false)
        .message_logger(Some(messages.clone()));
    cc::emit_link_directives(&build, test.td.path().join("libfoo.a"));
    assert!(test.get_cpp_stdlib_probes(0).is_some(), "no probe ran");
    let messages = messages.0.lock().unwrap();
    assert!(
        messages
            .iter()
            .all(|msg| !msg.contains("detect_cpp_stdlib") && !msg.contains("cannot preprocess")),
        "{messages:?}"
    );
}

/// A clone shares the flag support cache with the `Build` it was cloned from,
/// so an answer may only be reused for a probe that would run the same way.
#[test]
fn flag_support_cache_is_per_build_env() {
    let test = Test::gnu();
    let build = test.gcc();
    // Without the override the shim accepts the flag.
    assert!(build.is_flag_supported("-Wprobed").unwrap());

    let mut rejecting = build.clone();
    rejecting.env("CC_SHIM_FAIL_IF_ARG", "-Wprobed");
    // A fresh `Build` with the same environment rejects it.
    assert!(!test
        .gcc()
        .env("CC_SHIM_FAIL_IF_ARG", "-Wprobed")
        .is_flag_supported("-Wprobed")
        .unwrap());
    assert!(
        !rejecting.is_flag_supported("-Wprobed").unwrap(),
        "the clone reused the answer probed with a different `Build::env`"
    );
}

#[test]
fn flag_support_cache_is_per_target() {
    let test = Test::gnu();
    let mut build = test.gcc();
    // One compiler for both targets, like `CC=clang`, that rejects the flag
    // only when it compiles for x86.
    build
        .compiler(test.td.path().join("cc"))
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .env("CC_SHIM_FAIL_IF_ARG", "-m32");
    assert!(build.is_flag_supported("-Wprobed").unwrap());

    let mut x86 = build.clone();
    x86.target("i686-unknown-linux-gnu");
    // A fresh `Build` for x86 rejects it.
    let mut fresh = test.gcc();
    fresh
        .compiler(test.td.path().join("cc"))
        .target("i686-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .env("CC_SHIM_FAIL_IF_ARG", "-m32");
    assert!(!fresh.is_flag_supported("-Wprobed").unwrap());
    assert!(
        !x86.is_flag_supported("-Wprobed").unwrap(),
        "the clone reused the answer probed for another target"
    );
}

#[test]
fn flag_support_cache_is_per_language() {
    let mut test = Test::gnu();
    // Only a C++ probe reads `CXXFLAGS`.
    test.env.set("CXXFLAGS", "-Dprobed_as_cpp");
    let mut build = test.gcc();
    build
        .compiler(test.td.path().join("cc"))
        .env("CC_SHIM_FAIL_IF_ARG", "-Dprobed_as_cpp");
    assert!(build.is_flag_supported("-Wprobed").unwrap());

    let mut cpp = build.clone();
    cpp.cpp(true);
    assert!(
        !cpp.is_flag_supported("-Wprobed").unwrap(),
        "the clone reused the answer probed for C"
    );
}

/// The probe also depends on the inherited environment, which a clone made
/// before the first read takes on its own.
#[test]
fn flag_support_cache_is_per_process_env() {
    let mut test = Test::gnu();
    test.env.remove("CC_SHIM_FAIL_IF_ARG");
    let mut build = test.gcc();
    build.compiler(test.td.path().join("cc"));
    let later = build.clone();
    assert!(build.is_flag_supported("-Wprobed").unwrap());
    test.env.set("CC_SHIM_FAIL_IF_ARG", "-Wprobed");
    assert!(
        !later.is_flag_supported("-Wprobed").unwrap(),
        "the answer was reused for a clone that inherited another environment"
    );
}

/// A `Build` reads the process environment on first use, not when it is made.
#[test]
fn env_snapshot_is_taken_on_first_read() {
    let mut test = Test::gnu();
    let mut build = test.gcc();
    test.env.set("CFLAGS", "-Dset_after_new");
    build.file("foo.c").compile("foo");
    test.cmd(0).must_have("-Dset_after_new");
}

/// Changes after the first read reach neither cc's own lookups nor the child
/// processes, so both agree with the flag support cache key.
#[test]
fn env_snapshot_ignores_later_changes() {
    let mut test = Test::gnu();
    test.env.remove("CC_SHIM_FAIL_IF_ARG");
    let mut build = test.gcc();
    build.file("foo.c").flag_if_supported("-Wprobed");
    build.try_get_compiler().unwrap();
    // Read by cc.
    test.env.set("CFLAGS", "-Dset_after_first_read");
    // Read by the compiler: the shim fails on this argument.
    test.env.set("CC_SHIM_FAIL_IF_ARG", "-Wprobed");
    build.try_compile("foo").unwrap();
    test.cmd(0)
        .must_have("-Wprobed")
        .must_not_have("-Dset_after_first_read");
}

/// A child process gets the snapshot's value of a variable changed later.
#[test]
fn env_snapshot_restores_changed_variables() {
    let mut test = Test::gnu();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "-Dnot_passed");
    let mut build = test.gcc();
    build.file("foo.c");
    build.try_get_compiler().unwrap();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "foo.c");
    build.try_compile("foo").unwrap();
}

/// `Build::env` wins over the snapshot, also when set after the first read.
#[test]
fn env_snapshot_is_overridden_by_build_env() {
    let mut test = Test::gnu();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "foo.c");
    let mut build = test.gcc();
    build.file("foo.c");
    build.try_get_compiler().unwrap();
    build.env("CC_SHIM_FAIL_IF_ARG", "-Dnot_passed");
    build.try_compile("foo").unwrap();
}

/// A clone made before the first read takes its own snapshot, one made after
/// keeps the snapshot of the original.
#[test]
fn env_snapshot_of_clones() {
    let mut test = Test::gnu();
    let mut build = test.gcc();
    build.file("foo.c");
    let before = build.clone();
    build.try_get_compiler().unwrap();
    test.env.set("CFLAGS", "-Dset_after_first_read");
    let after = build.clone();

    after.compile_intermediates();
    test.cmd(0).must_not_have("-Dset_after_first_read");
    before.compile_intermediates();
    test.cmd(1).must_have("-Dset_after_first_read");
}

/// The `PATH` that MSVC's tool environment puts its folders in front of wins
/// over the snapshot's.
#[cfg(windows)]
#[test]
fn env_snapshot_keeps_msvc_tool_env() {
    use std::ffi::OsStr;

    let mut test = Test::new();
    let mut build = cc::Build::new();
    build
        .target("x86_64-pc-windows-msvc")
        .host("x86_64-pc-windows-msvc")
        .opt_level(0)
        .debug(false)
        .out_dir(test.td.path());
    let path_of = |envs: Vec<(&OsStr, Option<&OsStr>)>| {
        envs.into_iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("PATH"))
            .map(|(_, value)| value.map(OsStr::to_owned))
            .collect::<Vec<_>>()
    };
    let tool = build.get_compiler();
    let tool_path = path_of(tool.env().iter().map(|(k, v)| (&**k, Some(&**v))).collect());
    if tool_path.is_empty() {
        // No Visual Studio found.
        return;
    }
    test.env.set("PATH", test.td.path());
    let cmd = build.get_compiler().to_command();
    assert_eq!(path_of(cmd.get_envs().collect()), tool_path);
}

/// The archiver also runs in the snapshot.
#[test]
fn env_snapshot_reaches_archiver() {
    let mut test = Test::gnu();
    let mut build = test.gcc();
    build.file("foo.c").ar_flag("--marker");
    build.try_get_compiler().unwrap();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "--marker");
    build.try_compile("foo").unwrap();
}

/// The flag support probe reads the snapshot of the `Build` it probes for, and
/// runs in it.
#[test]
fn env_snapshot_reaches_flag_support_probe() {
    let mut test = Test::gnu();
    test.env.remove("CC_SHIM_FAIL_IF_ARG");
    test.collect_flag_supported_probes();
    let build = test.gcc();
    build.try_get_compiler().unwrap();
    test.env.set("CFLAGS", "-Dset_after_first_read");
    test.env.set("CC_SHIM_FAIL_IF_ARG", "-Wprobed");
    assert!(build.is_flag_supported("-Wprobed").unwrap());
    test.get_flag_supported_probes(0)
        .must_have("-Wprobed")
        .must_not_have("-Dset_after_first_read");
}

/// For a cross target, cc runs `<prefix>-ar --version` to pick the archiver,
/// in the snapshot too.
#[test]
fn env_snapshot_reaches_cross_archiver_probe() {
    let mut test = Test::gnu();
    test.shim("aarch64-linux-gnu-ar");
    for var in [
        "CROSS_COMPILE",
        "RUSTC_LINKER",
        "AR_aarch64-unknown-linux-gnu",
        "AR_aarch64_unknown_linux_gnu",
        "TARGET_AR",
        "CC_SHIM_FAIL_IF_ARG",
    ] {
        test.env.remove(var);
    }
    // The probe doesn't get `Build::env`, so the shim has to be on the
    // inherited `PATH`.
    test.env.set("PATH", test.td.path());
    let mut build = test.gcc();
    build
        .target("aarch64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu");
    build.try_get_compiler().unwrap();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "--version");
    let archiver = build.try_get_archiver().unwrap();
    assert_eq!(archiver.get_program(), "aarch64-linux-gnu-ar");
}

/// For Android, cc runs `llvm-ar --version` to pick the archiver, in the
/// snapshot too.
#[test]
fn env_snapshot_reaches_android_archiver_probe() {
    let mut test = Test::gnu();
    test.shim("llvm-ar");
    for var in [
        "AR_aarch64-linux-android",
        "AR_aarch64_linux_android",
        "TARGET_AR",
        "CC_SHIM_FAIL_IF_ARG",
    ] {
        test.env.remove(var);
    }
    let mut build = test.gcc();
    build
        .target("aarch64-linux-android")
        .host("x86_64-unknown-linux-gnu")
        .compiler(test.td.path().join("cc"));
    build.try_get_compiler().unwrap();
    test.env.set("CC_SHIM_FAIL_IF_ARG", "--version");
    let archiver = build.try_get_archiver().unwrap();
    assert_eq!(archiver.get_program(), "llvm-ar");
}

/// For Android, cc looks for the compiler by running each candidate name, in
/// the snapshot too.
#[test]
fn env_snapshot_reaches_android_compiler_probe() {
    let mut test = Test::gnu();
    test.shim("aarch64-linux-android-gcc");
    for var in [
        "CC_aarch64-linux-android",
        "CC_aarch64_linux_android",
        "TARGET_CC",
    ] {
        test.env.remove(var);
    }
    // The probe doesn't get `Build::env`, so the shim has to be on the
    // inherited `PATH`.
    test.env.set("PATH", test.td.path());
    let mut build = test.gcc();
    build
        .target("aarch64-linux-android")
        .host("x86_64-unknown-linux-gnu");
    build.try_get_compiler().unwrap();
    test.env.set("PATH", test.td.path().join("empty"));
    let compiler = build.try_get_compiler().unwrap();
    // Not found, cc would fall back to `aarch64-linux-android-clang`. On
    // Windows it also turns either name into `gcc.exe` or `clang.exe`.
    let name = compiler.path().file_stem().unwrap().to_str().unwrap();
    assert!(name.ends_with("gcc"), "{name}");
}

/// Compiler family detection runs the compiler in the snapshot too.
#[test]
fn env_snapshot_reaches_family_detection() {
    let mut test = Test::gnu();
    // The probe resolves `cc` through the inherited `PATH`, so this `Build`
    // doesn't set one with `Build::env`.
    test.env.set("PATH", test.td.path());
    let probe_record = test.td.path().join("family-detection");
    let mut build = cc::Build::new();
    build
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .opt_level(2)
        .out_dir(test.td.path())
        .compiler("cc")
        .env("CC_SHIM_OUT_FILES_FOR_FAMILY_DETECTION", &probe_record);
    build.try_flags_from_environment("CFLAGS").unwrap();
    test.env.set("PATH", test.td.path().join("empty"));
    build.try_get_compiler().unwrap();
    assert!(probe_record.exists());
}

/// Error messages show the command, not the environment cc sets on it.
#[test]
fn env_snapshot_stays_out_of_error_messages() {
    let mut test = Test::gnu();
    test.env.set("CC_TEST_SECRET", "env_snapshot_secret_value");
    test.env.set("CC_SHIM_FAIL_IF_ARG", "foo.c");
    let error = test
        .gcc()
        .file("foo.c")
        // Forwarded stderr is written to stdout directly, past the test
        // harness's capture.
        .cargo_warnings(false)
        .try_compile("foo")
        .unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("command did not execute successfully"),
        "{message}"
    );
    assert!(!message.contains("env_snapshot_secret_value"), "{message}");
}

/// The host decides whether the probe reads `HOST_CFLAGS` or `TARGET_CFLAGS`.
#[test]
fn flag_support_cache_is_per_host() {
    let mut test = Test::gnu();
    for var in [
        "CC_SHIM_FAIL_IF_ARG",
        "CFLAGS_x86_64-unknown-linux-gnu",
        "CFLAGS_x86_64_unknown_linux_gnu",
        "HOST_CFLAGS",
    ] {
        test.env.remove(var);
    }
    let mut build = test.gcc();
    build
        .compiler(test.td.path().join("cc"))
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu");
    test.env.set("TARGET_CFLAGS", "-m32");
    build.env("CC_SHIM_FAIL_IF_ARG", "-m32");
    assert!(build.is_flag_supported("-Wprobed").unwrap());
    let mut cross = build.clone();
    cross.host("aarch64-unknown-linux-gnu");
    assert!(
        !cross.is_flag_supported("-Wprobed").unwrap(),
        "the clone reused the answer probed for another host"
    );
}

#[test]
fn gnu_create_archive() {
    let test = Test::gnu();
    let extra = test.td.path().join("extra.o");
    let mut build = test.gcc();
    build.file("foo.c").object(&extra);
    let objects = build.compile_intermediates();
    let library = build.try_create_archive("foo", &objects).unwrap();

    assert_eq!(library, test.td.path().join("libfoo.a"));
    // The objects passed in come first, then those added with `object`.
    test.cmd(1)
        .must_have("cqD")
        .must_have(&library)
        .must_have_in_order(objects[0].to_str().unwrap(), extra.to_str().unwrap());
    test.cmd(2).must_have("sD").must_have(&library);

    // The deprecated `lib<name>.a` form names the same file.
    let library = build.try_create_archive("libfoo.a", &objects).unwrap();
    assert_eq!(library, test.td.path().join("libfoo.a"));
}

#[test]
fn msvc_create_archive() {
    let test = Test::msvc();
    let mut build = test.gcc();
    build.file("foo.c");
    let objects = build.compile_intermediates();
    let library = build.try_create_archive("foo", &objects).unwrap();

    assert_eq!(library, test.td.path().join("libfoo.a"));
    let mut out = std::ffi::OsString::from("-out:");
    out.push(&library);
    test.cmd(1).must_have(out).must_have(&objects[0]);
    // As with `compile`, the library is also available as `foo.lib`.
    assert!(test.td.path().join("foo.lib").is_file());
}

#[test]
fn msvc_llvm_ar_name_ignores_case() {
    let test = Test::msvc();
    test.shim("LLVM-AR");
    test.gcc()
        .archiver(test.td.path().join("LLVM-AR"))
        .file("foo.c")
        .compile("foo");

    // llvm-ar takes `ar` style arguments, not `lib.exe` style ones.
    test.cmd(1)
        .must_run("LLVM-AR")
        .must_have("cqD")
        .must_not_have("-nologo");
}

#[test]
fn msvc_llvm_ar_check_ignores_folder_name() {
    let test = Test::msvc();
    let llvm_bin = test.td.path().join("llvm-arm64").join("bin");
    std::fs::create_dir_all(&llvm_bin).unwrap();
    let llvm_lib = format!("llvm-lib{}", env::consts::EXE_SUFFIX);
    std::fs::copy(&test.gcc, llvm_bin.join(llvm_lib)).unwrap();
    test.gcc()
        .archiver(llvm_bin.join("llvm-lib"))
        .file("foo.c")
        .compile("foo");

    // Only the file name tells whether the archiver is llvm-ar.
    test.cmd(1)
        .must_run("llvm-lib")
        .must_have("-nologo")
        .must_not_have("cqD");
}

#[test]
fn create_archive_checks_its_arguments() {
    let test = Test::gnu();
    let build = test.gcc();
    for output in ["", "a/b", "..", "/foo"] {
        let err = build
            .try_create_archive(output, ["foo.o"])
            .unwrap_err()
            .to_string();
        assert!(
            err.starts_with("InvalidArgument: argument of `create_archive`"),
            "{output:?}: {err}"
        );
    }
}

#[test]
fn create_archive_respects_cc_force_disable() {
    let mut test = Test::gnu();
    test.env.set("CC_FORCE_DISABLE", "1");
    let err = test
        .gcc()
        .try_create_archive("foo", ["foo.o"])
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("Disabled: "), "{err}");
    assert!(!test.td.path().join("out0").exists(), "the archiver ran");
}

#[test]
fn emit_link_directives_checks_its_arguments() {
    {
        let test = Test::gnu();
        let mut build = test.gcc();
        build.cargo_metadata(false);
        for library in [
            "out/foo.a",
            "out/lib.a",
            "out/libfoo.so",
            "libfoo.a",
            "out/foo.lib",
        ] {
            let err = cc::try_emit_link_directives(&build, library)
                .unwrap_err()
                .to_string();
            assert!(
                err.starts_with("InvalidArgument: `emit_link_directives` expects"),
                "{library:?}: {err}"
            );
        }
        cc::try_emit_link_directives(&build, "out/libfoo.a").unwrap();
    }

    // `<name>.lib` is only a static library name on MSVC targets.
    let test = Test::msvc();
    let mut build = test.gcc();
    build.cargo_metadata(false).cargo_warnings(false);
    cc::try_emit_link_directives(&build, "out/foo.lib").unwrap();
    cc::try_emit_link_directives(&build, "out/libfoo.a").unwrap();
}

/// `compile_intermediates`, `create_archive` and `emit_link_directives` run
/// the same archiver commands and emit the same link lines as `compile`.
///
/// This test runs the builds in a subprocess so we
/// can capture and assert on the emitted cargo metadata.
#[test]
fn create_archive_and_emit_link_directives_match_compile() {
    // When invoked as subprocess, perform the builds and return.
    if let Some(case) = env::var_os("__CC_TEST_SPLIT_COMPILE") {
        let case = case.into_string().unwrap();
        let (case, how) = case.split_once(' ').unwrap();
        // `cuda` builds a CUDA file, which also device-links the archive.
        let cuda = case == "cuda";
        let target = if cuda {
            "x86_64-unknown-linux-gnu"
        } else {
            case
        };
        let msvc = target.ends_with("-msvc");
        let test = if msvc { Test::msvc() } else { Test::gnu() };
        test.shim("clang++").shim("nvcc");
        let mut build = test.gcc();
        build
            .target(target)
            .host(target)
            .cpp(true)
            .cpp_link_stdlib_static(true)
            .link_lib_modifier("+whole-archive");
        if !msvc {
            build.archiver(test.td.path().join("ar"));
        }
        if cuda {
            build
                .cuda(true)
                .compiler(test.td.path().join("nvcc"))
                .file("foo.cu");
        } else {
            build.file("foo.cpp");
            if target != "x86_64-unknown-linux-gnu" && !msvc {
                build.compiler(test.td.path().join("clang++"));
            }
        }
        // The test shim always creates `libfoo.a`, which MSVC needs to copy.
        let names: &[&str] = if msvc { &["foo"] } else { &["foo", "bar"] };
        for name in names {
            if how == "compile" {
                build.compile(name);
            } else {
                let objects = build.compile_intermediates();
                let library = build.create_archive(name, &objects);
                if how == "split-lib" {
                    cc::emit_link_directives(&build, library.with_file_name(format!("{name}.lib")));
                } else {
                    cc::emit_link_directives(&build, &library);
                }
            }
        }
        for i in 0.. {
            let Ok(args) = std::fs::read_to_string(test.td.path().join(format!("out{i}"))) else {
                break;
            };
            println!("ran: {}", args.lines().collect::<Vec<_>>().join(" "));
        }
        println!("out-dir: {}", test.td.path().display());
        return;
    }

    let run = |target: &str, how: &str| -> Vec<String> {
        let output = GlobalEnv::output(
            Command::new(env::current_exe().unwrap())
                .env("__CC_TEST_SPLIT_COMPILE", format!("{target} {how}"))
                .env("PAUTHTEST_SYSROOT", "sysroot")
                .env("PAUTHTEST_RESOURCE_DIR", "resource-dir")
                .env_remove("CXXSTDLIB")
                .env_remove("CXXSTDLIB_STATIC")
                .env_remove("CC_ENABLE_DEBUG_OUTPUT")
                .args([
                    "--exact",
                    "create_archive_and_emit_link_directives_match_compile",
                    "--nocapture",
                ]),
        );
        assert!(output.status.success(), "subprocess failed: {:?}", output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let out_dir = stdout
            .lines()
            .find_map(|line| line.strip_prefix("out-dir: "))
            .unwrap();
        let stdout = stdout.replace(out_dir, "<out-dir>");
        stdout
            .lines()
            .filter(|line| {
                line.starts_with("cargo:rustc-")
                    || line.starts_with("cargo:rerun-if-env-changed=")
                    // The shim can't answer family detection, and the failed
                    // probe's file name is random.
                    || (line.starts_with("cargo:warning=")
                        && !line.contains("detect_compiler_family"))
                    || line.starts_with("ran: ")
            })
            .map(str::to_owned)
            .collect()
    };

    let link_lib_lines = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter_map(|line| line.strip_prefix("cargo:rustc-link-lib="))
            .map(str::to_owned)
            .collect()
    };

    for (target, link_libs) in [
        (
            "x86_64-unknown-linux-gnu",
            &[
                "static:+whole-archive=foo",
                "static:-bundle=stdc++",
                "static:+whole-archive=bar",
            ][..],
        ),
        (
            "aarch64-unknown-linux-pauthtest",
            &[
                "static:+whole-archive=foo",
                "static=c++",
                "static:+whole-archive=bar",
                "static=c++",
            ][..],
        ),
        (
            "cuda",
            &[
                "static:+whole-archive=foo",
                "static:-bundle=stdc++",
                "cudart_static",
                "static:+whole-archive=bar",
                "cudart_static",
            ][..],
        ),
        ("x86_64-pc-windows-msvc", &["static:+whole-archive=foo"][..]),
    ] {
        let compile = run(target, "compile");
        assert_eq!(link_lib_lines(&compile), link_libs, "{target}");
        if target == "cuda" {
            assert!(
                compile.iter().any(|line| line.contains("--device-link")),
                "{compile:#?}"
            );
        }
        let mut hows = vec!["split"];
        if target.ends_with("-msvc") {
            hows.push("split-lib");
        }
        for how in hows {
            assert_eq!(run(target, how), compile, "{target} {how}");
        }
    }
}
