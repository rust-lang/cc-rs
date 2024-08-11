use gen_target_info::{get_target_specs_from_json, write_target_tuple_mapping, RustcTargetSpecs};
use std::{fs::File, io::Write as _};

const PRELUDE: &str = r#"//! This file is generated code. Please edit the generator
//! in dev-tools/gen-target-info if you need to make changes.

"#;

fn generate_riscv_arch_mapping(f: &mut File, target_specs: &RustcTargetSpecs) {
    let mut riscv_target_mapping = target_specs
        .0
        .iter()
        .filter_map(|(target, target_spec)| {
            let arch = target.split_once('-').unwrap().0;
            (arch.contains("riscv") && arch != target_spec.arch)
                .then_some((arch, &*target_spec.arch))
        })
        .collect::<Vec<_>>();
    riscv_target_mapping.sort_unstable_by_key(|(arch, _)| &**arch);
    riscv_target_mapping.dedup();
    write_target_tuple_mapping(f, "RISCV_ARCH_MAPPING", &riscv_target_mapping);
}

fn generate_windows_triple_mapping(f: &mut File, target_specs: &RustcTargetSpecs) {
    let mut windows_target_mapping = target_specs
        .0
        .iter()
        .filter_map(|(target, target_spec)| {
            let rust_target_parts = target.splitn(4, '-').collect::<Vec<_>>();
            let os = *rust_target_parts.get(2)?;
            (os.contains("windows") && target != &*target_spec.llvm_target)
                .then_some((&**target, &*target_spec.llvm_target))
        })
        .collect::<Vec<_>>();
    windows_target_mapping.sort_unstable_by_key(|(triple, _)| &**triple);
    windows_target_mapping.dedup();
    write_target_tuple_mapping(f, "WINDOWS_TRIPLE_MAPPING", &windows_target_mapping);
}

fn main() {
    let target_specs = get_target_specs_from_json();

    // Open file to write to
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let path = format!("{manifest_dir}/../../src/target_info.rs");
    let mut f = File::create(path).expect("failed to create src/target_info.rs");

    f.write_all(PRELUDE.as_bytes()).unwrap();

    // Start generating
    generate_riscv_arch_mapping(&mut f, &target_specs);
    generate_windows_triple_mapping(&mut f, &target_specs);

    // Flush the data onto disk
    f.flush().unwrap();
}
