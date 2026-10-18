//! Compiles `src/heap.c` without any sanitizer flag of its own, so the C code
//! is only instrumented if cc passes on the sanitizer from `CARGO_CFG_SANITIZE`.

fn main() {
    println!("cargo:rerun-if-changed=src/heap.c");
    cc::Build::new().file("src/heap.c").compile("heap");
}
