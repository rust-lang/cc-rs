//! Compiles `src/heap.c` and `src/tables.c` without any sanitizer flag of their
//! own, so the C code is only instrumented if cc passes on the sanitizer from
//! `CARGO_CFG_SANITIZE`.

fn main() {
    println!("cargo:rerun-if-changed=src/heap.c");
    println!("cargo:rerun-if-changed=src/tables.c");
    cc::Build::new().file("src/heap.c").compile("heap");
    // GCC merges identical read-only variables from `-O2` on.
    cc::Build::new()
        .file("src/tables.c")
        .opt_level(2)
        .compile("tables");
}
