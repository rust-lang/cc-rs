use std::{fs::File, io::Write as _};

const PRELUDE: &str = r#"//! This file is generated code. Please edit the generator
//! in dev-tools/gen-target-info if you need to make changes.

"#;

fn main() {
    // Open file to write to
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let path = format!("{manifest_dir}/../../src/target_info.rs");
    let mut f = File::create(path).expect("failed to create src/target_info.rs");

    f.write_all(PRELUDE.as_bytes()).unwrap();

    // Start generating
    // TODO

    // Flush the data onto disk
    f.flush().unwrap();
}
