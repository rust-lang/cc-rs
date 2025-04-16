#[test]
fn main() {
    let cfg = cc_with_target("i586-pc-nto-qnx700");
    assert_eq!(cfg.get_archiver().get_program(), "ntox86-ar");

    let cfg = cc_with_target("x86_64-unknown-linux-gnu");
    assert_eq!(cfg.get_archiver().get_program(), "ar");

    let cfg = cc_with_target("x86_64-unknown-linux-musl");
    assert_eq!(cfg.get_archiver().get_program(), "ar");

    let cfg = cc_with_target("riscv64gc-unknown-openbsd");
    assert_eq!(cfg.get_archiver().get_program(), "ar");

    let cfg = cc_with_target("i686-wrs-vxworks");
    assert_eq!(cfg.get_archiver().get_program(), "wr-ar");

    let cfg = cc_with_target("i586-pc-nto-qnx700");
    assert_eq!(cfg.get_archiver().get_program(), "ntox86-ar");

    let cfg = cc_with_target("aarch64-unknown-nto-qnx700");
    assert_eq!(cfg.get_archiver().get_program(), "ntoaarch64-ar");

    let cfg = cc_with_target("x86_64-pc-nto-qnx710");
    assert_eq!(cfg.get_archiver().get_program(), "ntox86_64-ar");

    let cfg = cc_with_target("wasm32-wasip1");
    // This usually returns an absolute path, so using `assert_eq` might make the test flaky.
    assert!(cfg
        .get_archiver()
        .get_program()
        .to_str()
        .unwrap()
        .ends_with("llvm-ar"));

    let cfg = cc_with_target("riscv64-linux-android");
    assert_eq!(cfg.get_archiver().get_program(), "llvm-ar");
}

fn cc_with_target(target: &'static str) -> cc::Build {
    let mut cfg = cc::Build::new();
    cfg.host("x86_64-unknown-linux-gnu").target(target);
    cfg
}
