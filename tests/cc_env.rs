extern crate tempdir;
extern crate cc;

use std::env;

mod support;
use support::Test;

#[test]
fn main() {
    ccache();
    distcc();
    ccache_spaces();
    ccache_env_flags();
}

fn ccache() {
    let test = Test::gnu();

    env::set_var("CC", "ccache cc");
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("foo.c")
        .must_not_have("ccache");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

fn ccache_spaces() {
    let test = Test::gnu();
    test.shim("ccache");

    env::set_var("CC", "ccache        cc");
    test.gcc().file("foo.c").compile("libfoo.a");
    test.cmd(0).must_have("foo.c");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

fn distcc() {
    let test = Test::gnu();
    test.shim("distcc");

    env::set_var("CC", "distcc cc");
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("foo.c")
        .must_not_have("distcc");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

fn ccache_env_flags() {
    use std::path::Path;
    use std::ffi::OsString;

    let test = Test::gnu();
    test.shim("ccache");

    env::set_var("CC", "ccache lol-this-is-not-a-compiler");
    let compiler = test.gcc().file("foo.c").get_compiler();
    assert_eq!(compiler.path(), Path::new("lol-this-is-not-a-compiler"));
    assert_eq!(compiler.cc_env(), OsString::from("ccache lol-this-is-not-a-compiler"));
    assert!(compiler.cflags_env().into_string().unwrap().contains("ccache") == false);
    assert!(compiler.cflags_env().into_string().unwrap().contains(" lol-this-is-not-a-compiler") == false);

    env::set_var("CC", "");
}
