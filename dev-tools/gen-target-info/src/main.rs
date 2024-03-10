use gen_target_info::{get_target_specs_from_json, write_target_tuple_mapping};
use std::{fs::File, io::Write as _};

fn main() {
    let target_specs = get_target_specs_from_json();

    // Open file to write to
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let path = format!("{manifest_dir}/../../src/target_info.rs");
    let mut f = File::create(path).expect("failed to create src/target_info.rs");

    // Wrute riscv target mapping
    let mut riscv_target_mapping = target_specs
        .0
        .iter()
        .filter_map(|(target, target_spec)| {
            let arch = target.split_once('-').unwrap().0;
            (arch.contains("riscv") && arch != &target_spec.arch)
                .then_some((arch, &*target_spec.arch))
        })
        .collect::<Vec<_>>();
    riscv_target_mapping.sort_unstable_by_key(|(arch, _)| &**arch);
    riscv_target_mapping.dedup();
    write_target_tuple_mapping(&mut f, "RISCV_ARCH_MAPPING", &riscv_target_mapping);

    // Flush the data onto disk
    f.flush().unwrap();
}
