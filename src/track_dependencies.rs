use std::{error::Error, fs, fs::File, iter, path::Path, process::Command, time::SystemTime};

use crate::Object;

pub enum WriteFileStatus {
    NewContentsWriten,
    NoWrite,
}

pub fn write_file_if_changed<P: AsRef<Path>>(
    path: P,
    content: &str,
) -> Result<WriteFileStatus, Box<dyn Error>> {
    let s = match fs::read_to_string(path.as_ref()) {
        Ok(s) => s,
        Err(_) => {
            fs::write(path.as_ref(), content)?;
            return Ok(WriteFileStatus::NewContentsWriten);
        }
    };

    if s != content {
        fs::write(path.as_ref(), content)?;
        return Ok(WriteFileStatus::NewContentsWriten);
    }

    Ok(WriteFileStatus::NoWrite)
}

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

fn dependencies(obj: &Object) -> Option<Vec<String>> {
    let deps_info_path = obj.dst.with_extension("json");

    if !deps_info_path.is_file() {
        return None;
    }

    let deps_info = match std::fs::read_to_string(deps_info_path) {
        Ok(res) => res,
        Err(_) => return None,
    };

    let deps = match json::parse(&deps_info) {
        Ok(res) => res,
        Err(_) => return None,
    };

    if !deps.has_key("Data") {
        return None;
    }

    let data = &deps["Data"];

    if !data.has_key("Includes") {
        return None;
    }

    let includes = &data["Includes"];

    let src_file = match obj.src.to_str() {
        Some(s) => s,
        None => return None,
    };

    Some(
        includes
            .members()
            .filter_map(|v| v.as_str())
            .chain(iter::once(src_file))
            .map(|s| s.to_string())
            .collect(),
    )
}

pub(crate) fn is_run_needed(obj: &Object, cmd: &Command) -> bool {
    match write_file_if_changed(obj.dst.with_extension("command"), &format!("{:?}", cmd)) {
        Ok(WriteFileStatus::NewContentsWriten) | Err(_) => return true,
        _ => (),
    }

    match dependencies(&obj) {
        Some(dependencies) => is_any_input_newer_then_output(&obj.dst, dependencies),
        None => true,
    }
}

pub(crate) fn emit_rerun_directives(obj: &Object) {
    if let Some(dependencies) = dependencies(&obj) {
        for dep in dependencies {
            println!("cargo:rerun-if-changed={}", dep);
        }
    }
}
