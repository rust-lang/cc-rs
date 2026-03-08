use std::env;
use std::ffi::OsString;
use std::path::Path;

mod support;
use crate::support::Test;

#[test]
fn ccache() {
    let mut test = Test::gnu();

    test.env.set("CC", "ccache cc");
    let compiler = test.gcc().file("foo.c").get_compiler();

    assert_eq!(compiler.path(), Path::new("cc"));
}

#[test]
fn ccache_spaces() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", "ccache        cc");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("cc"));
}

#[test]
fn distcc() {
    let mut test = Test::gnu();
    test.shim("distcc");

    test.env.set("CC", "distcc cc");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("cc"));
}

#[test]
fn ccache_env_flags() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", "ccache lol-this-is-not-a-compiler");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("lol-this-is-not-a-compiler"));
    assert_eq!(
        compiler.cc_env(),
        OsString::from("ccache lol-this-is-not-a-compiler")
    );
    assert!(!compiler
        .cflags_env()
        .into_string()
        .unwrap()
        .contains("ccache"));
    assert!(!compiler
        .cflags_env()
        .into_string()
        .unwrap()
        .contains(" lol-this-is-not-a-compiler"));

    test.env.set("CC", "");
}

#[test]
fn leading_spaces() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", " test ");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("test"));

    test.env.set("CC", "");
}

#[test]
fn extra_flags() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", "ccache cc -m32");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("cc"));
}

#[test]
fn path_to_ccache() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", "/path/to/ccache.exe cc -m32");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("cc"));
    assert_eq!(
        compiler.cc_env(),
        OsString::from("/path/to/ccache.exe cc -m32"),
    );
}

#[test]
fn more_spaces() {
    let mut test = Test::gnu();
    test.shim("ccache");

    test.env.set("CC", "cc -m32");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("cc"));
}

#[test]
fn clang_cl() {
    for exe_suffix in ["", ".exe"] {
        let mut test = Test::clang();
        let bin = format!("clang{exe_suffix}");
        test.env.set("CC", format!("{bin} --driver-mode=cl"));
        let test_compiler = |build: cc::Build| {
            let compiler = build.get_compiler();
            assert_eq!(compiler.path(), Path::new(&*bin));
            assert!(compiler.is_like_msvc());
            assert!(compiler.is_like_clang_cl());
        };
        test_compiler(test.gcc());
    }
}

#[test]
fn env_var_alternatives_override() {
    let compiler1 = format!("clang1{}", env::consts::EXE_SUFFIX);
    let compiler2 = format!("clang2{}", env::consts::EXE_SUFFIX);
    let compiler3 = format!("clang3{}", env::consts::EXE_SUFFIX);
    let compiler4 = format!("clang4{}", env::consts::EXE_SUFFIX);
    let compiler5 = format!("clang5{}", env::consts::EXE_SUFFIX);

    let mut test = Test::new();
    test.shim(&compiler1);
    test.shim(&compiler2);
    test.shim(&compiler3);
    test.shim(&compiler4);
    test.shim(&compiler5);

    test.env.set("CC", &compiler1);
    let compiler = test.gcc().target("x86_64-unknown-none").get_compiler();
    assert_eq!(compiler.path(), Path::new(&compiler1));

    test.env.set("HOST_CC", &compiler2);
    test.env.set("TARGET_CC", &compiler2);
    let compiler = test.gcc().target("x86_64-unknown-none").get_compiler();
    assert_eq!(compiler.path(), Path::new(&compiler2));

    test.env.set("CC_x86_64_unknown_none", &compiler3);
    let compiler = test.gcc().target("x86_64-unknown-none").get_compiler();
    assert_eq!(compiler.path(), Path::new(&compiler3));

    test.env.set("CC_x86_64-unknown-none", &compiler4);
    let compiler = test.gcc().target("x86_64-unknown-none").get_compiler();
    assert_eq!(compiler.path(), Path::new(&compiler4));

    test.env.set("CC_thumbv8m_main_none_eabi", &compiler5);
    let compiler = test.gcc().target("thumbv8m.main-none-eabi").get_compiler();
    assert_eq!(compiler.path(), Path::new(&compiler5));
}
