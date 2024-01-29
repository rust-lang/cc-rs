extern "C" {
    pub fn foo() -> i32;

    pub fn bar1() -> i32;
    pub fn bar2() -> i32;

    pub fn asm() -> i32;

    pub fn baz() -> i32;

    #[cfg(windows)]
    pub fn windows();

    #[cfg(target_env = "msvc")]
    pub fn msvc();
}
