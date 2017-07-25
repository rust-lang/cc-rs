extern crate gcc_test;

use gcc_test::*;

use std::path::Path;
use std::process::{Command, Stdio};

#[link(name = "OptLinkage", kind = "static")]
extern "C" {
    fn answer() -> i32;
}

#[test]
fn hello_works() {
    let child = Command::new(Path::new(env!("OUT_DIR")).join("hello"))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute hello.");
    let output = child.wait_with_output().expect("Failed to wait on hello.").stdout;
    let output = String::from_utf8(output).unwrap();

    assert_eq!(output, "Hello World!");
}

#[test]
fn foo_here() {
    unsafe {
        assert_eq!(foo(), 4);
    }
}

#[test]
fn bar_here() {
    unsafe {
        assert_eq!(bar1(), 5);
        assert_eq!(bar2(), 6);
    }
}

#[test]
fn asm_here() {
    unsafe {
        assert_eq!(asm(), 7);
    }
}

#[test]
fn baz_here() {
    unsafe {
        assert_eq!(baz(), 8);
    }
}

#[test]
#[cfg(windows)]
fn windows_here() {
    unsafe {
        windows();
    }
}

#[test]
#[cfg(target_env = "msvc")]
fn msvc_here() {
    unsafe {
        msvc();
    }
}

#[test]
fn opt_linkage() {
    unsafe {
        assert_eq!(answer(), 42);
    }
}
