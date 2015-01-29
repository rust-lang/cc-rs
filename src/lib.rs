#![feature(collections, core, io, path, os)]

use std::old_io::Command;
use std::old_io::process::InheritFd;
use std::default::Default;

/// Extra configuration to pass to gcc.
pub struct Config {
    /// Directories where gcc will look for header files.
    pub include_directories: Vec<Path>,
    /// Additional definitions (`-DKEY` or `-DKEY=VALUE`).
    pub definitions: Vec<(String, Option<String>)>,
    /// Additional object files to link into the final archive
    pub objects: Vec<Path>,
    /// Additional flags and parameter to pass to the compiler
    pub flags: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            include_directories: Vec::new(),
            definitions: Vec::new(),
            objects: Vec::new(),
            flags: Vec::new(),
        }
    }
}

fn getenv(v: &str) -> Option<String> {
    use std::os::getenv;
    let r = getenv(v);
    println!("{} = {:?}", v, r);
    r
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

    let target = getenv("TARGET").unwrap();
    let opt_level = getenv("OPT_LEVEL").unwrap();

    let mut cmd = Command::new(gcc(target.as_slice()));
    cmd.arg(format!("-O{}", opt_level));
    cmd.arg("-c");
    cmd.arg("-ffunction-sections").arg("-fdata-sections");
    cmd.args(cflags().as_slice());

    if target.as_slice().contains("-ios") {
        cmd.args(ios_flags(target.as_slice()).as_slice());
    } else {
        if target.contains("windows") {
            cmd.arg("-mwin32");
        }

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

    for flag in config.flags.iter() {
        cmd.arg(flag);
    }

    for &(ref key, ref value) in config.definitions.iter() {
        if let &Some(ref value) = value {
            cmd.arg(format!("-D{}={}", key, value));
        } else {
            cmd.arg(format!("-D{}", key));
        }
    }

    let src = Path::new(getenv("CARGO_MANIFEST_DIR").unwrap());
    let dst = Path::new(getenv("OUT_DIR").unwrap());
    let mut objects = Vec::new();
    for file in files.iter() {
        let obj = dst.join(*file).with_extension("o");
        std::old_io::fs::mkdir_recursive(&obj.dir_path(), std::old_io::USER_RWX).unwrap();
        run(cmd.clone().arg(src.join(*file)).arg("-o").arg(&obj));
        objects.push(obj);
    }


    run(Command::new(ar(target.as_slice())).arg("crus")
                                           .arg(dst.join(output))
                                           .args(objects.as_slice())
                                           .args(config.objects.as_slice()));
    println!("cargo:rustc-flags=-L native={} -l static={}",
             dst.display(), output.slice(3, output.len() - 2));
}

fn run(cmd: &mut Command) {
    println!("running: {:?}", cmd);
    let status = match cmd.stdout(InheritFd(1)).stderr(InheritFd(2)).status() {
        Ok(status) => status,
        Err(e) => panic!("failed to spawn process: {}", e),
    };
    if !status.success() {
        panic!("nonzero exit status: {}", status);
    }
}

fn get_var(var_base: &str) -> Result<String, String> {
    let target = getenv("TARGET")
        .expect("Environment variable 'TARGET' is unset");
    let host = match getenv("HOST") {
            None => { return Err("Environment variable 'HOST' is unset".to_string()); }
            Some(x) => x
        };
    let kind = if host == target { "HOST" } else { "TARGET" };
    let target_u = target.split('-')
        .collect::<Vec<&str>>()
        .connect("_");
    let res = getenv(format!("{}_{}", var_base, target).as_slice())
        .or_else(|| getenv(format!("{}_{}", var_base, target_u).as_slice()))
        .or_else(|| getenv(format!("{}_{}", kind, var_base).as_slice()))
        .or_else(|| getenv(var_base));

    match res {
        Some(res) => Ok(res),
        None => Err("Could not get environment variable".to_string()),
    }
}

fn gcc(target: &str) -> String {
    let is_android = target.find_str("android").is_some();

    get_var("CC").unwrap_or(if cfg!(windows) {
        "gcc".to_string()
    } else if is_android {
        format!("{}-gcc", target)
    } else {
        "cc".to_string()
    })
}

fn ar(target: &str) -> String {
    let is_android = target.find_str("android").is_some();

    get_var("AR").unwrap_or(if is_android {
        format!("{}-ar", target)
    } else {
        "ar".to_string()
    })
}

fn cflags() -> Vec<String> {
    get_var("CFLAGS").unwrap_or(String::new())
       .as_slice().words().map(|s| s.to_string())
       .collect()
}

fn ios_flags(target: &str) -> Vec<String> {
    enum ArchSpec {
        Device(&'static str),
        Simulator(&'static str),
    }

    let mut res = Vec::new();

    let arch = target.split('-').nth(0).expect("expected target in format `arch-vendor-os`");
    let arch = match arch {
        "arm" | "armv7" | "thumbv7" => ArchSpec::Device("armv7"),
        "armv7s" | "thumbv7s" => ArchSpec::Device("armv7s"),
        "arm64" | "aarch64" => ArchSpec::Device("aarch64"),
        "i386" | "i686" => ArchSpec::Simulator("-m32"),
        "x86_64" => ArchSpec::Simulator("-m64"),
        _ => unreachable!("Unknown arch for iOS target")
    };

    let sdk = match arch {
        ArchSpec::Device(arch) => {
            res.push("-arch".to_string());
            res.push(arch.to_string());
            "iphoneos"
        },
        ArchSpec::Simulator(arch) => {
            res.push(arch.to_string());
            "iphonesimulator"
        }
    };

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

    res.push("-isysroot".to_string());
    res.push(sdk_path.trim().to_string());

    res
}
