extern crate gcc;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::remove_dir_all(&out).unwrap();
    fs::create_dir(&out).unwrap();

    gcc::Config::new()
        .file("src/foo.c")
        .define("FOO", None)
        .define("BAR", Some("1"))
        .compile("libfoo.a");

    gcc::Config::new()
        .file("src/bar1.c")
        .file("src/bar2.c")
        .include("src/include")
        .compile("libbar.a");

    let target = std::env::var("TARGET").unwrap();
    let file = target.split("-").next().unwrap();
    let file = format!("src/{}.{}",
                       file,
                       if target.contains("msvc") { "asm" } else { "S" });
    gcc::Config::new()
        .file(file)
        .compile("libasm.a");

    gcc::Config::new()
        .file("src/baz.cpp")
        .cpp(true)
        .compile("libbaz.a");

    if target.contains("windows") {
        gcc::Config::new()
            .file("src/windows.c")
            .compile("libwindows.a");
    }

    // Test that the `windows_registry` module will set PATH by looking for
    // nmake which runs vanilla cl, and then also test it after we remove all
    // the relevant env vars from our own process.
    if target.contains("msvc") {
        let out = out.join("tmp");
        fs::create_dir(&out).unwrap();
        println!("nmake 1");
        let status = gcc::windows_registry::find(&target, "nmake.exe")
            .unwrap()
            .arg("/fsrc/NMakefile")
            .env("OUT_DIR", &out)
            .status()
            .unwrap();
        assert!(status.success());

        fs::remove_dir_all(&out).unwrap();
        fs::create_dir(&out).unwrap();

        env::remove_var("PATH");
        env::remove_var("VCINSTALLDIR");
        env::remove_var("INCLUDE");
        env::remove_var("LIB");
        println!("nmake 2");
        let status = gcc::windows_registry::find(&target, "nmake.exe")
            .unwrap()
            .arg("/fsrc/NMakefile")
            .env("OUT_DIR", &out)
            .status()
            .unwrap();
        assert!(status.success());
        println!("cargo:rustc-link-lib=msvc");
        println!("cargo:rustc-link-search={}", out.display());
    }

    // This tests whether we  can build a library but not link it to the main
    // crate.  The test module will do its own linking.
    gcc::Config::new()
        .cargo_metadata(false)
        .file("src/opt_linkage.c")
        .compile("libOptLinkage.a");
}
