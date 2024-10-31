use std::io::Write as _;
use std::{fs::File, io::BufRead};

use gen_target_info::{
    get_target_spec_from_msrv, get_target_specs_from_json, get_targets_msrv, RustcTargetSpecs,
};

const PRELUDE: &str = r#"//! This file is generated code. Please edit the generator
//! in dev-tools/gen-target-info if you need to make changes.

"#;

fn generate_target_mapping(f: &mut File, target_specs: &RustcTargetSpecs) -> std::io::Result<()> {
    writeln!(f, "use super::Target;")?;
    writeln!(f, "use std::borrow::Cow;")?;
    writeln!(f)?;
    writeln!(f, "pub(crate) const LIST: &[(&str, Target)] = &[")?;

    for (triple, spec) in &target_specs.0 {
        let full_arch = triple.split_once('-').unwrap().0;
        let arch = &spec.arch;
        let vendor = spec.vendor.as_deref().unwrap_or("unknown");
        let os = spec.os.as_deref().unwrap_or("none");
        let env = spec.env.as_deref().unwrap_or("");
        let abi = spec.abi.as_deref().unwrap_or("");

        writeln!(f, "    (")?;
        writeln!(f, "        {triple:?},")?;
        writeln!(f, "        Target {{")?;
        writeln!(f, "            full_arch: Cow::Borrowed({full_arch:?}),")?;
        writeln!(f, "            arch: Cow::Borrowed({arch:?}),")?;
        writeln!(f, "            vendor: Cow::Borrowed({vendor:?}),")?;
        writeln!(f, "            os: Cow::Borrowed({os:?}),")?;
        writeln!(f, "            env: Cow::Borrowed({env:?}),")?;
        writeln!(f, "            abi: Cow::Borrowed({abi:?}),")?;
        writeln!(f, "        }},")?;
        writeln!(f, "    ),")?;
    }

    writeln!(f, "];")?;

    Ok(())
}

fn main() {
    // Primarily use information from nightly.
    let mut target_specs = get_target_specs_from_json();
    // Next, read from MSRV to support old, removed targets.
    for target_triple in get_targets_msrv().lines() {
        let target_triple = target_triple.unwrap();
        let target_triple = target_triple.trim();
        target_specs
            .0
            .entry(target_triple.to_string())
            .or_insert_with(|| get_target_spec_from_msrv(target_triple));
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
