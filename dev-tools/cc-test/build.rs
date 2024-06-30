#![allow(clippy::disallowed_methods)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // if we are being executed from a `fork_run_action` call (i.e. this is a
    // "fork"), perform the requested action and then return.
    if run_action_if_forked() {
        return;
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::remove_dir_all(&out).unwrap();
    fs::create_dir(&out).unwrap();

    // The following are builds where we want to capture the output (i.e. stdout and
    // stderr). We do that by re-running _this_ executable and passing in the
    // action as the first argument.
    run_forked_capture_output(&out, "metadata-on");
    run_forked_capture_output(&out, "metadata-off");

    run_forked_capture_output(&out, "warnings-off");
    if cc::Build::new().get_compiler().is_like_msvc() {
        // MSVC doesn't output warnings to stderr, so we can't capture them.
        // the test will use this env var to know whether to run the test.
        println!("cargo:rustc-env=TEST_WARNINGS_ON=0");
    } else {
        println!("cargo:rustc-env=TEST_WARNINGS_ON=1");
        run_forked_capture_output(&out, "warnings-on");
    }

    let mut build = cc::Build::new();
    build
        .file("src/foo.c")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wfoo-bar-this-flag-does-not-exist")
        .define("FOO", None)
        .define("BAR", "1")
        .compile("foo");

    let compiler = build.get_compiler();

    cc::Build::new()
        .file("src/bar1.c")
        .file("src/bar2.c")
        .include("src/include")
        .compile("bar");

    let target = std::env::var("TARGET").unwrap();
    let file = target.split('-').next().unwrap();
    let file = format!(
        "src/{}.{}",
        file,
        if target.contains("msvc") { "asm" } else { "S" }
    );
    cc::Build::new().file(file).compile("asm");

    cc::Build::new()
        .file("src/baz.cpp")
        .cpp(true)
        .compile("baz");

    if env::var("CARGO_FEATURE_TEST_CUDA").is_ok() {
        // Detect if there is CUDA compiler and engage "cuda" feature.
        let nvcc = match env::var("NVCC") {
            Ok(var) => which::which(var),
            Err(_) => which::which("nvcc"),
        };
        if nvcc.is_ok() {
            cc::Build::new()
                .cuda(true)
                .cudart("static")
                .file("src/cuda.cu")
                .compile("libcuda.a");

            // Communicate [cfg(feature = "cuda")] to test/all.rs.
            println!("cargo:rustc-cfg=feature=\"cuda\"");
        }
    }

    if target.contains("windows") {
        cc::Build::new().file("src/windows.c").compile("windows");
    }

    if target.contains("msvc") {
        let cc_frontend = if compiler.is_like_msvc() {
            "MSVC"
        } else if compiler.is_like_clang() {
            "CLANG"
        } else {
            unimplemented!("Unknown compiler that targets msvc but isn't clang-like or msvc-like")
        };

        // Test that the `windows_registry` module will set PATH by looking for
        // nmake which runs vanilla cl, and then also test it after we remove all
        // the relevant env vars from our own process.
        let out = out.join("tmp");
        fs::create_dir(&out).unwrap();
        println!("nmake 1");
        let status = cc::windows_registry::find(&target, "nmake.exe")
            .unwrap()
            .env_remove("MAKEFLAGS")
            .arg("/fsrc/NMakefile")
            .env("OUT_DIR", &out)
            .env("CC_FRONTEND", cc_frontend)
            .status()
            .unwrap();
        assert!(status.success());

        fs::remove_dir_all(&out).unwrap();
        fs::create_dir(&out).unwrap();

        // windows registry won't find clang in path
        if !compiler.path().to_string_lossy().starts_with("clang") {
            env::remove_var("PATH");
        }
        env::remove_var("VCINSTALLDIR");
        env::remove_var("INCLUDE");
        env::remove_var("LIB");
        println!("nmake 2");
        let status = cc::windows_registry::find(&target, "nmake.exe")
            .unwrap()
            .env_remove("MAKEFLAGS")
            .arg("/fsrc/NMakefile")
            .env("OUT_DIR", &out)
            .env("CC_FRONTEND", cc_frontend)
            .status()
            .unwrap();
        assert!(status.success());
        println!("cargo:rustc-link-lib=msvc");
        println!("cargo:rustc-link-search={}", out.display());

        // Test that the `windows_registry` module detects if we're in a "spectre
        // mode" VS environment.
        fn has_spectre(target: &str) -> bool {
            cc::windows_registry::find_tool(target, "cl.exe")
                .unwrap()
                .env()
                .iter()
                .any(|(k, v)| (k == "LIB") && v.to_str().unwrap().contains(r"\lib\spectre\"))
        }

        std::env::set_var("VSCMD_ARG_VCVARS_SPECTRE", "spectre");
        assert!(
            has_spectre(&target),
            "LIB should use spectre-mitigated libs when VSCMD_ARG_VCVARS_SPECTRE is set"
        );

        std::env::remove_var("VSCMD_ARG_VCVARS_SPECTRE");
        assert!(
            !has_spectre(&target),
            "LIB should not use spectre-mitigated libs when VSCMD_ARG_VCVARS_SPECTRE is not set"
        );
    }

    // This tests whether we  can build a library but not link it to the main
    // crate.  The test module will do its own linking.
    cc::Build::new()
        .cargo_metadata(false)
        .file("src/opt_linkage.c")
        .compile("OptLinkage");

    let out = cc::Build::new().file("src/expand.c").expand();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("hello world"));
}

#[track_caller]
fn run_forked_capture_output(out: &Path, action: &str) {
    let program = env::current_exe().unwrap();
    let output = Command::new(program).arg(action).output().unwrap();
    assert!(output.status.success(), "output: {:#?}", output);
    // we've captured the output and now we write it to a dedicated directory in the
    // build output so our tests can access the output.
    let action_dir = out.join(action);
    fs::create_dir_all(&action_dir).unwrap();
    fs::write(action_dir.join("stdout"), output.stdout).unwrap();
    fs::write(action_dir.join("stderr"), output.stderr).unwrap();
}

fn run_action_if_forked() -> bool {
    let mut args = env::args();
    let _program = args.next().unwrap();
    let action = args.next();
    match action.as_deref() {
        Some("metadata-on") => build_cargo_metadata(true),
        Some("metadata-off") => build_cargo_metadata(false),
        Some("warnings-on") => build_cargo_warnings(true),
        Some("warnings-off") => build_cargo_warnings(false),
        // No action requested, we're being called from cargo. Proceed with build.
        _ => return false,
    }
    true
}

fn disable_debug_output() {
    // That env would break tests for warning/debug output,
    // and it is set in the CI, to make debugging CI failure easier.
    std::env::remove_var("CC_ENABLE_DEBUG_OUTPUT");
}

fn build_cargo_warnings(warnings: bool) {
    disable_debug_output();

    cc::Build::new()
        .cargo_metadata(false)
        .cargo_warnings(warnings)
        .file("src/compile_error.c")
        .try_compile("compile_error")
        .unwrap_err();
}

fn build_cargo_metadata(metadata: bool) {
    disable_debug_output();

    cc::Build::new()
        .cargo_metadata(metadata)
        .file("src/dummy.c")
        .try_compile("dummy")
        .unwrap();
}
