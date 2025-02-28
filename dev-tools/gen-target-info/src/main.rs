use std::io::Write as _;
use std::{fs::File, io::BufRead};

use gen_target_info::{
    get_target_spec_from_msrv, get_target_specs_from_json, get_targets_msrv, RustcTargetSpecs,
};

const PRELUDE: &str = r#"//! This file is generated code. Please edit the generator in
//! dev-tools/gen-target-info if you need to make changes, or see
//! src/target/llvm.rs if you need to configure a specific LLVM triple.

"#;

fn generate_target_mapping(f: &mut File, target_specs: &RustcTargetSpecs) -> std::io::Result<()> {
    writeln!(f, "#[rustfmt::skip]")?;
    writeln!(f, "pub(crate) const LLVM_TARGETS: &[(&str, &str)] = &[")?;

    for (triple, spec) in &target_specs.0 {
        writeln!(f, "    ({triple:?}, {:?}),", spec.llvm_target)?;
    }

    writeln!(f, "];")?;

    Ok(())
}

fn main() {
    // Primarily use information from nightly.
    let mut target_specs = get_target_specs_from_json(std::env::var("RUSTC").ok());
    // Next, read from MSRV to support old, removed targets.
    if std::env::var("CC_RS_MSRV")
        .unwrap_or("1".to_string())
        .parse::<u32>()
        .unwrap()
        != 0
    {
        for target_triple in get_targets_msrv().lines() {
            let target_triple = target_triple.unwrap();
            let target_triple = target_triple.trim();
            target_specs
                .0
                .entry(target_triple.to_string())
                .or_insert_with(|| get_target_spec_from_msrv(target_triple));
        }
    }

    // Open file to write to
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let path = format!("{manifest_dir}/../../src/target/generated.rs");
    let mut f = File::create(path).expect("failed to create src/target/generated.rs");

    f.write_all(PRELUDE.as_bytes()).unwrap();

    // Start generating
    generate_target_mapping(&mut f, &target_specs).unwrap();

    // Flush the data onto disk
    f.flush().unwrap();
}
