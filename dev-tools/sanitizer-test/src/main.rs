//! Calls into C code that cc compiled, to check in CI that the C code works
//! with the sanitizer the Rust code is built with. With the argument
//! `overflow`, the C code reads past the end of a heap buffer, which the
//! sanitizer must report. It also reads two identical tables. GCC merges them
//! unless cc turns that off, and the sanitizer reports the merged table as an
//! ODR violation.

extern "C" {
    fn read_heap(index: usize) -> i32;
    fn table_entry(index: usize) -> i32;
    fn table_entry_from_end(index: usize) -> i32;
}

fn main() {
    // SAFETY: Both take an index below 16.
    let (first, last) = unsafe { (table_entry(0), table_entry_from_end(0)) };
    assert_eq!((first, last), (3, 95));

    let index = match std::env::args().nth(1).as_deref() {
        Some("overflow") => 8,
        _ => 7,
    };
    // SAFETY: `read_heap` takes any index. Only 8 reads out of bounds, on
    // purpose, for the sanitizer to report.
    let value = unsafe { read_heap(index) };
    println!("read {value}");
}
