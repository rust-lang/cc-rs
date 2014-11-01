use std::os;
use std::io::Command;
use std::io::process::InheritFd;

/// Compile a library from the given set of input C files.
///
/// This will simply compile all files into object files and then assemble them
/// into the output. This will read the standard environment variables to detect
/// cross compilations and such.
///
/// # Example
///
/// ```
/// gcc::compile_library("libfoo.a", &["foo.c", "bar.c"]);
/// ```
pub fn compile_library(output: &str, files: &[&str]) {
    assert!(output.starts_with("lib"));
    assert!(output.ends_with(".a"));

    let target = os::getenv("TARGET").unwrap();
    let opt_level = os::getenv("OPT_LEVEL").unwrap();

    let mut cmd = Command::new(gcc());
    cmd.arg(format!("-O{}", opt_level));
    cmd.arg("-c");
    cmd.arg("-ffunction-sections").arg("-fdata-sections");
    cmd.args(cflags().as_slice());

    if target.as_slice().contains("i686") {
        cmd.arg("-m32");
    } else if target.as_slice().contains("x86_64") {
        cmd.arg("-m64");
    }

    let src = Path::new(os::getenv("CARGO_MANIFEST_DIR").unwrap());
    let dst = Path::new(os::getenv("OUT_DIR").unwrap());
    let mut objects = Vec::new();
    for file in files.iter() {
        let obj = dst.join(*file).with_extension("o");
        run(cmd.clone().arg(src.join(*file)).arg("-o").arg(&obj));
        objects.push(obj);
    }


    run(Command::new(ar()).arg("crus")
                          .arg(dst.join(output))
                          .args(objects.as_slice()));
    println!("cargo:rustc-flags=-L {} -l {}",
             dst.display(), output.slice(3, output.len() - 2));
}

fn run(cmd: &mut Command) {
    println!("running: {}", cmd);
    assert!(cmd.stdout(InheritFd(1))
               .stderr(InheritFd(2))
               .status()
               .unwrap()
               .success());

}

fn gcc() -> String {
    os::getenv("CC").unwrap_or(if cfg!(windows) {
        "gcc".to_string()
    } else {
        "cc".to_string()
    })
}

fn ar() -> String {
    os::getenv("AR").unwrap_or("ar".to_string())
}

fn cflags() -> Vec<String> {
    os::getenv("CFLAGS").unwrap_or(String::new())
       .as_slice().words().map(|s| s.to_string())
       .collect()
}
