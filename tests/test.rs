use crate::support::Test;

mod support;

// Some tests check that a flag is *not* present.  These tests might fail if the flag is set in the
// CFLAGS or CXXFLAGS environment variables.  This function clears the CFLAGS and CXXFLAGS
// variables to make sure that the tests can run correctly.
fn reset_env() {
    std::env::set_var("CFLAGS", "");
    std::env::set_var("CXXFLAGS", "");
}

#[test]
fn gnu_smoke() {
    reset_env();

    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-O2")
        .must_have("foo.c")
        .must_not_have("-gdwarf-4")
        .must_have("-c")
        .must_have("-ffunction-sections")
        .must_have("-fdata-sections");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

#[test]
fn gnu_opt_level_1() {
    reset_env();

    let test = Test::gnu();
    test.gcc().opt_level(1).file("foo.c").compile("foo");

    test.cmd(0).must_have("-O1").must_not_have("-O2");
}

#[test]
fn gnu_opt_level_s() {
    reset_env();

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
        .target("x86_64-unknown-linux")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");

    let test = Test::gnu();
    test.gcc()
        .target("x86_64-apple-darwin")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-2");
}

#[test]
fn gnu_debug_fp_auto() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_have("-fno-omit-frame-pointer");
}

#[test]
fn gnu_debug_fp() {
    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux")
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_have("-fno-omit-frame-pointer");
}

#[test]
fn gnu_debug_nofp() {
    reset_env();

    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux")
        .debug(true)
        .force_frame_pointer(false)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_not_have("-fno-omit-frame-pointer");

    let test = Test::gnu();
    test.gcc()
        .target("x86_64-unknown-linux")
        .force_frame_pointer(false)
        .debug(true)
        .file("foo.c")
        .compile("foo");
    test.cmd(0).must_have("-gdwarf-4");
    test.cmd(0).must_not_have("-fno-omit-frame-pointer");
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
fn gnu_extra_warnings0() {
    reset_env();

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
    reset_env();

    let test = Test::gnu();
    test.gcc()
        .warnings(false)
        .extra_warnings(true)
        .flag("-Wno-missing-field-initializers")
        .file("foo.c")
        .compile("foo");

    test.cmd(0).must_not_have("-Wall").must_have("-Wextra");
}

#[test]
fn gnu_warnings_overridable() {
    reset_env();

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
    reset_env();

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
fn gnu_set_stdlib() {
    reset_env();

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
    reset_env();

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
    reset_env();

    if cfg!(windows) {
        return;
    }
    let test = Test::gnu();
    test.gcc()
        .file("foo.c")
        .flag("-v")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wflag-does-not-exist")
        .flag_if_supported("-std=c++11")
        .compile("foo");

    test.cmd(0)
        .must_have("-v")
        .must_have("-Wall")
        .must_not_have("-Wflag-does-not-exist")
        .must_not_have("-std=c++11");
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
    reset_env();

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
    reset_env();

    let test = Test::msvc();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("-O2")
        .must_have("foo.c")
        .must_not_have("-Z7")
        .must_have("-c")
        .must_have("-MD");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

#[test]
fn msvc_opt_level_0() {
    reset_env();

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
fn gnu_apple_darwin() {
    for (arch, version) in &[("x86_64", "10.7"), ("aarch64", "11.0")] {
        let target = format!("{}-apple-darwin", arch);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            // Avoid test maintainence when minimum supported OSes change.
            .__set_env("MACOSX_DEPLOYMENT_TARGET", version)
            .file("foo.c")
            .compile("foo");

        let cmd = test.cmd(0);
        test.cmd(0)
            .must_have(format!("-mmacosx-version-min={}", version));
        cmd.must_not_have("-isysroot");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_cpp_minimums() {
    let versions = &[
        // Too low
        ("10.7", "10.9"),
        // Minimum
        ("10.9", "10.9"),
        // Higher
        ("11.0", "11.0"),
    ];

    let target = "x86_64-apple-darwin";
    for (deployment_target, expected) in versions {
        let test = Test::gnu();
        test.gcc()
            .target(target)
            .host(target)
            .cpp(true)
            .__set_env("MACOSX_DEPLOYMENT_TARGET", deployment_target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0)
            .must_have(format!("-mmacosx-version-min={}", expected));
    }

    let test = Test::gnu();
    test.gcc()
        .target(target)
        .host(target)
        .__set_env("MACOSX_DEPLOYMENT_TARGET", "10.7")
        .file("foo.c")
        .compile("foo");

    // No C++ leaves it untouched
    test.cmd(0).must_have("-mmacosx-version-min=10.7");
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_tvos() {
    for target in &["aarch64-apple-tvos"] {
        let test = Test::clang();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_have("-mappletvos-version-min=9.0");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn clang_apple_tvsimulator() {
    for target in &["x86_64-apple-tvos"] {
        let test = Test::clang();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c")
            .compile("foo");

        test.cmd(0).must_have("-mappletvsimulator-version-min=9.0");
    }
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
