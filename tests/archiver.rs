mod support;

#[test]
fn targe_ar_env() {
    let mut env = support::GlobalEnv::lock();
    env.set("AR_i586_pc_nto_qnx700", "custom-ar");
    let ar = get_ar_for_target("i586-pc-nto-qnx700");
    assert_eq!(ar, "custom-ar");
}

#[test]
fn ar_env() {
    let mut env = support::GlobalEnv::lock();
    env.set("AR", "custom-ar2");
    let ar = get_ar_for_target("x86_64-unknown-linux-gnu");
    assert_eq!(ar, "custom-ar2");
}

#[test]
fn various() {
    let mut env = support::GlobalEnv::lock();
    env.remove("AR");

    let ar = get_ar_for_target("x86_64-unknown-linux-gnu");
    assert_eq!(ar, "ar");

    let ar = get_ar_for_target("x86_64-unknown-linux-musl");
    assert_eq!(ar, "ar");

    let ar = get_ar_for_target("riscv64gc-unknown-openbsd");
    assert_eq!(ar, "ar");

    let ar = get_ar_for_target("i686-wrs-vxworks");
    assert_eq!(ar, "wr-ar");

    let ar = get_ar_for_target("i586-pc-nto-qnx700");
    assert_eq!(ar, "ntox86-ar");

    let ar = get_ar_for_target("aarch64-unknown-nto-qnx700");
    assert_eq!(ar, "ntoaarch64-ar");

    let ar = get_ar_for_target("x86_64-pc-nto-qnx710");
    assert_eq!(ar, "ntox86_64-ar");

    let ar = get_ar_for_target("wasm32-wasip1");
    assert!(
        // `llvm-ar` is usually an absolute path for this target, so we check it with `ends_with`.
        ar.ends_with(&maybe_exe("llvm-ar"))
        // If `llvm-ar` doesn't exist, the logic falls back to `ar` for this target.
        || ar == "ar"
    );

    let ar = get_ar_for_target("riscv64-linux-android");
    // If `llvm-ar` is not available on the system, this will fall back to `$target-ar` (e.g., `riscv64-linux-android-ar` in this case)
    assert!(ar == "llvm-ar" || ar == "riscv64-linux-android-ar");
}

fn get_ar_for_target(target: &'static str) -> String {
    let mut cfg = cc::Build::new();
    cfg.host("x86_64-unknown-linux-gnu").target(target);
    let ar = cfg.get_archiver();
    let ar = ar.get_program().to_str().unwrap().to_string();
    println!("cc::Build::get_archiver -> target: '{target}': resolved archiver: '{ar}'");
    ar
}

/// Appends `.exe` to the file name on Windows systems.
fn maybe_exe(file: &'static str) -> String {
    if cfg!(windows) {
        format!("{file}.exe")
    } else {
        file.to_owned()
    }
}
