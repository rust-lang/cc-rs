//! Calls into C code that cc compiled, to check in CI that the C code works
//! with the sanitizer the Rust code is built with. With the argument
//! `overflow`, the C code reads past the end of a heap buffer, which the
//! sanitizer must report.

extern "C" {
    fn read_heap(index: usize) -> i32;
}

fn main() {
    let index = match std::env::args().nth(1).as_deref() {
        Some("overflow") => 8,
        _ => 7,
    };
    // SAFETY: `read_heap` takes any index. Only 8 reads out of bounds, on
    // purpose, for the sanitizer to report.
    let value = unsafe { read_heap(index) };
    println!("read {value}");
}
