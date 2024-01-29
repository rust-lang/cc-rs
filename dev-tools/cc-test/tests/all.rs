use cc_test::*;

#[link(name = "OptLinkage", kind = "static")]
extern "C" {
    fn answer() -> i32;
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

#[cfg(feature = "cuda")]
#[test]
fn cuda_here() {
    extern "C" {
        fn cuda_kernel();
    }
    unsafe {
        cuda_kernel();
    }
}
