use crate::support::Test;
mod support;

/// This test is in its own module because it modifies the environment and would affect other tests
/// when run in parallel with them.
#[test]
#[cfg(not(windows))]
fn inherits_rustflags() {
    // Sanity check - no flags
    std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "");
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_not_have("-fno-omit-frame-pointer")
        .must_not_have("-mcmodel=small")
        .must_not_have("-msoft-float");

    // Correctly inherits flags from rustc
    std::env::set_var(
        "CARGO_ENCODED_RUSTFLAGS",
        "-Cforce-frame-pointers=true\u{1f}-Ccode-model=small\u{1f}-Csoft-float",
    );
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");
    test.cmd(0)
        .must_have("-fno-omit-frame-pointer")
        .must_have("-mcmodel=small")
        .must_have("-msoft-float");
}
