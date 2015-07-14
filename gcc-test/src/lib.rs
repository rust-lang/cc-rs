extern {
    pub fn foo() -> i32;
}

extern {
    pub fn bar1() -> i32;
    pub fn bar2() -> i32;
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
