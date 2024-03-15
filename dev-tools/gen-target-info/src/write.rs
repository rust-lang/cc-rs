use std::{fmt::Write as _, fs, io::Write as _};

pub fn write_target_tuple_mapping(f: &mut fs::File, variable_name: &str, data: &[(&str, &str)]) {
    let mut content = format!("pub const {variable_name}: &[(&str, &str)] = &[\n");

    for (f1, f2) in data {
        write!(&mut content, r#"    ("{f1}", "{f2}"),"#).unwrap();
        content.push('\n');
    }

    content.push_str("];\n");

    f.write_all(content.as_bytes()).unwrap();
}
