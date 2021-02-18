use std::{fs::File, iter, path::Path, time::SystemTime};

use crate::Object;

fn get_modified_time<P: AsRef<Path>>(p: P) -> Option<SystemTime> {
    let f = File::open(p).ok()?;
    let metadata = f.metadata().ok()?;
    metadata.modified().ok()
}

pub fn is_any_input_newer_then_output<P1: AsRef<Path>, P2: AsRef<Path>>(
    out_path: P1,
    in_paths: impl IntoIterator<Item = P2>,
) -> bool {
    let out_time = get_modified_time(out_path.as_ref());

    if out_time.is_none() {
        return true;
    }

    for in_path in in_paths.into_iter() {
        let in_time = get_modified_time(in_path.as_ref());

        if in_time.is_none() {
            return true;
        }

        if in_time.unwrap() >= out_time.unwrap() {
            return true;
        }
    }

    false
}

pub(crate) fn is_run_needed(obj: &Object) -> bool {
    let deps_info_path = obj.dst.with_extension("json");

    if !deps_info_path.is_file() {
        return true;
    }

    let deps_info = match std::fs::read_to_string(deps_info_path) {
        Ok(res) => res,
        Err(_) => return true,
    };

    let deps = match json::parse(&deps_info) {
        Ok(res) => res,
        Err(_) => return true,
    };

    if !deps.has_key("Data") {
        return true;
    }

    let data = &deps["Data"];

    if !data.has_key("Includes") {
        return true;
    }

    let includes = &data["Includes"];

    let src_file = match obj.src.to_str() {
        Some(s) => s,
        None => return true,
    };

    is_any_input_newer_then_output(
        &obj.dst,
        includes
            .members()
            .filter_map(|v| v.as_str())
            .chain(iter::once(src_file)),
    )
}
