#![feature(if_let)]

use std::os;
use std::io::Command;
use std::io::process::InheritFd;
use std::default::Default;

/// Extra configuration to pass to gcc.
pub struct Config {
    /// Directories where gcc will look for header files.
    pub include_directories: Vec<Path>,
    /// Additional definitions (`-DKEY` or `-DKEY=VALUE`).
    pub definitions: Vec<(String, Option<String>)>,
    /// Additional object files to link into the final archive
    pub objects: Vec<Path>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            include_directories: Vec::new(),
            definitions: Vec::new(),
            objects: Vec::new(),
        }
    }
}

/// Compile a library from the given set of input C files.
///
/// This will simply compile all files into object files and then assemble them
/// into the output. This will read the standard environment variables to detect
/// cross compilations and such.
///
/// # Example
///
/// ```no_run
/// use std::default::Default;
/// gcc::compile_library("libfoo.a", &Default::default(), &[
///     "foo.c",
///     "bar.c",
/// ]);
/// ```
pub fn compile_library(output: &str, config: &Config, files: &[&str]) {
    assert!(output.starts_with("lib"));
    assert!(output.ends_with(".a"));

    let target = os::getenv("TARGET").unwrap();
    let opt_level = os::getenv("OPT_LEVEL").unwrap();

    let mut cmd = Command::new(gcc());
    cmd.arg(format!("-O{}", opt_level));
    cmd.arg("-c");
    cmd.arg("-ffunction-sections").arg("-fdata-sections");
    cmd.args(cflags().as_slice());

    if target.as_slice().contains("-ios") {
        cmd.args(ios_flags(target.as_slice()).as_slice());
    } else {
        if target.as_slice().contains("i686") {
            cmd.arg("-m32");
        } else if target.as_slice().contains("x86_64") {
            cmd.arg("-m64");
        }

        if !target.as_slice().contains("i686") {
            cmd.arg("-fPIC");
        }
    }

    for directory in config.include_directories.iter() {
        cmd.arg("-I").arg(directory);
    }

    for &(ref key, ref value) in config.definitions.iter() {
        if let &Some(ref value) = value {
            cmd.arg(format!("-D{}={}", key, value));
        } else {
            cmd.arg(format!("-D{}", key));
        }
    }

    let src = Path::new(os::getenv("CARGO_MANIFEST_DIR").unwrap());
    let dst = Path::new(os::getenv("OUT_DIR").unwrap());
    let mut objects = Vec::new();
    for file in files.iter() {
        let obj = dst.join(*file).with_extension("o");
        std::io::fs::mkdir_recursive(&obj.dir_path(), std::io::USER_RWX).unwrap();
        run(cmd.clone().arg(src.join(*file)).arg("-o").arg(&obj));
        objects.push(obj);
    }


    run(Command::new(ar()).arg("crus")
                          .arg(dst.join(output))
                          .args(objects.as_slice())
                          .args(config.objects.as_slice()));
    println!("cargo:rustc-flags=-L {} -l {}:static",
             dst.display(), output.slice(3, output.len() - 2));
}

fn run(cmd: &mut Command) {
    println!("running: {}", cmd);
    let status = match cmd.stdout(InheritFd(1)).stderr(InheritFd(2)).status() {
        Ok(status) => status,
        Err(e) => panic!("failed to spawn process: {}", e),
    };
    if !status.success() {
        panic!("nonzero exit status: {}", status);
    }
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

fn ios_flags(target: &str) -> Vec<String> {
    let mut is_device_arch = false;
    let mut res = Vec::new();

    if target.starts_with("arm-") {
        res.push("-arch");
        res.push("armv7");
        is_device_arch = true;
    } else if target.starts_with("arm64-") {
        res.push("-arch");
        res.push("arm64");
        is_device_arch = true;
    } else if target.starts_with("i386-") {
        res.push("-m32");
    } else if target.starts_with("x86_64-") {
        res.push("-m64");
    }

    let sdk = if is_device_arch {"iphoneos"} else {"iphonesimulator"};

    println!("Detecting iOS SDK path for {}", sdk);
    let sdk_path = Command::new("xcrun")
        .arg("--show-sdk-path")
        .arg("--sdk")
        .arg(sdk)
        .stderr(InheritFd(2))
        .output()
        .unwrap()
        .output;

    let sdk_path = String::from_utf8(sdk_path).unwrap();

    res.push("-isysroot");
    res.push(sdk_path.as_slice().trim());

    res.iter().map(|s| s.to_string()).collect::<Vec<_>>()
}
