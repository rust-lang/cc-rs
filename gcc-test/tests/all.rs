extern crate gcc_test;

use gcc_test::*;

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
