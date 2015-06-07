//! A library for build scripts to compile custom C code
//!
//! This library is intended to be used as a `build-dependencies` entry in
//! `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! gcc = "0.2"
//! ```
//!
//! The purpose of this crate is to provide the utility functions necessary to
//! compile C code into a static archive which is then linked into a Rust crate.
//! The top-level `compile_library` function serves as a convenience and more
//! advanced configuration is available through the `Config` builder.
//!
//! This crate will automatically detect situations such as cross compilation or
//! other environment variables set by Cargo and will build code appropriately.
//!
//! # Examples
//!
//! Use the default configuration:
//!
//! ```no_run
//! extern crate gcc;
//!
//! fn main() {
//!     gcc::compile_library("libfoo.a", &["src/foo.c"]);
//! }
//! ```
//!
//! Use more advanced configuration:
//!
//! ```no_run
//! extern crate gcc;
//!
//! fn main() {
//!     gcc::Config::new()
//!                 .file("src/foo.c")
//!                 .define("FOO", Some("bar"))
//!                 .include("src")
//!                 .compile("libfoo.a");
//! }
//! ```

#![doc(html_root_url = "http://alexcrichton.com/gcc-rs")]
#![cfg_attr(test, deny(warnings))]

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};

/// Extra configuration to pass to gcc.
pub struct Config {
    include_directories: Vec<PathBuf>,
    definitions: Vec<(String, Option<String>)>,
    objects: Vec<PathBuf>,
    flags: Vec<String>,
    files: Vec<PathBuf>,
    cpp: bool,
    cpp_link_stdlib: Option<String>,
    cpp_set_stdlib: Option<String>,
}

/// Returns the default C++ standard library for the current target: `libc++`
/// for OS X and `libstdc++` for anything else.
fn target_default_cpp_stdlib() -> Option<&'static str> {
    let target = getenv_unwrap("TARGET");
    if target.contains("msvc") {
        None
    } else if target.contains("darwin") {
        Some("c++")
    } else {
        Some("stdc++")
    }
}

fn getenv(v: &str) -> Option<String> {
    let r = env::var(v).ok();
    println!("{} = {:?}", v, r);
    r
}

fn getenv_unwrap(v: &str) -> String {
    match getenv(v) {
        Some(s) => s,
        None => fail(&format!("environment variable `{}` not defined", v)),
    }
}

/// Compile a library from the given set of input C files.
///
/// This will simply compile all files into object files and then assemble them
/// into the output. This will read the standard environment variables to detect
/// cross compilations and such.
///
/// This function will also print all metadata on standard output for Cargo.
///
/// # Example
///
/// ```no_run
/// gcc::compile_library("libfoo.a", &["foo.c", "bar.c"]);
/// ```
pub fn compile_library(output: &str, files: &[&str]) {
    let mut c = Config::new();
    for f in files.iter() {
        c.file(*f);
    }
    c.compile(output)
}

impl Config {
    /// Construct a new instance of a blank set of configuration.
    ///
    /// This builder is finished with the `compile` function.
    pub fn new() -> Config {
        Config {
            include_directories: Vec::new(),
            definitions: Vec::new(),
            objects: Vec::new(),
            flags: Vec::new(),
            files: Vec::new(),
            cpp: false,
            cpp_link_stdlib: target_default_cpp_stdlib().map(|s| s.into()),
            cpp_set_stdlib: None,
        }
    }

    /// Add a directory to the `-I` or include path for headers
    pub fn include<P: AsRef<Path>>(&mut self, dir: P) -> &mut Config {
        self.include_directories.push(dir.as_ref().to_path_buf());
        self
    }

    /// Specify a `-D` variable with an optional value.
    pub fn define(&mut self, var: &str, val: Option<&str>) -> &mut Config {
        self.definitions.push((var.to_string(), val.map(|s| s.to_string())));
        self
    }

    /// Add an arbitrary object file to link in
    pub fn object<P: AsRef<Path>>(&mut self, obj: P) -> &mut Config {
        self.objects.push(obj.as_ref().to_path_buf());
        self
    }

    /// Add an arbitrary flag to the invocation of the compiler
    pub fn flag(&mut self, flag: &str) -> &mut Config {
        self.flags.push(flag.to_string());
        self
    }

    /// Add a file which will be compiled
    pub fn file<P: AsRef<Path>>(&mut self, p: P) -> &mut Config {
        self.files.push(p.as_ref().to_path_buf());
        self
    }

    /// Set C++ support.
    ///
    /// The other `cpp_*` options will only become active if this is set to
    /// `true`.
    pub fn cpp(&mut self, cpp: bool) -> &mut Config {
        self.cpp = cpp;
        self
    }

    /// Set the standard library to link against when compiling with C++
    /// support.
    ///
    /// The default value of this property depends on the current target: On
    /// OS X `Some("c++")` is used, when compiling for a Visual Studio based
    /// target `None` is used and for other targets `Some("stdc++")` is used.
    ///
    /// A value of `None` indicates that no automatic linking should happen,
    /// otherwise cargo will link against the specified library.
    ///
    /// The given library name must not contain the `lib` prefix.
    pub fn cpp_link_stdlib(&mut self, cpp_link_stdlib: Option<&str>) -> &mut Config {
        self.cpp_link_stdlib = cpp_link_stdlib.map(|s| s.into());
        self
    }

    /// Force the C++ compiler to use the specified standard library.
    ///
    /// Setting this option will automatically set `cpp_link_stdlib` to the same
    /// value.
    ///
    /// The default value of this option is always `None`.
    ///
    /// This option has no effect when compiling for a Visual Studio based
    /// target.
    ///
    /// This option sets the `-stdlib` flag, which is only supported by some
    /// compilers (clang, icc) but not by others (gcc). The library will not
    /// detect which compiler is used, as such it is the responsibility of the
    /// caller to ensure that this option is only used in conjuction with a
    /// compiler which supports the `-stdlib` flag.
    ///
    /// A value of `None` indicates that no specific C++ standard library should
    /// be used, otherwise `-stdlib` is added to the compile invocation.
    ///
    /// The given library name must not contain the `lib` prefix.
    pub fn cpp_set_stdlib(&mut self, cpp_set_stdlib: Option<&str>) -> &mut Config {
        self.cpp_set_stdlib = cpp_set_stdlib.map(|s| s.into());

        self.cpp_link_stdlib(cpp_set_stdlib);

        self
    }

    /// Run the compiler, generating the file `output`
    ///
    /// The name `output` must begin with `lib` and end with `.a`
    pub fn compile(&self, output: &str) {
        assert!(output.starts_with("lib"));
        assert!(output.ends_with(".a"));
        let lib_name = &output[3..output.len() - 2];

        let target = getenv_unwrap("TARGET");
        let src = PathBuf::from(getenv_unwrap("CARGO_MANIFEST_DIR"));
        let dst = PathBuf::from(getenv_unwrap("OUT_DIR"));
        let mut objects = Vec::new();
        for file in self.files.iter() {
            let mut cmd = self.compile_cmd(&target);
            cmd.arg(src.join(file));

            let obj = dst.join(file).with_extension("o");
            fs::create_dir_all(&obj.parent().unwrap()).unwrap();
            if target.contains("msvc") {
                let mut s = OsString::from("/Fo:");
                s.push(&obj);
                cmd.arg(s);
            } else {
                cmd.arg("-o").arg(&obj);
            }
            run(&mut cmd, &self.compiler(&target));
            objects.push(obj);
        }

        if target.contains("msvc") {
            let mut out = OsString::from("/OUT:");
            out.push(dst.join(output));
            run(Command::new("lib").arg(out).args(&objects).args(&self.objects),
                "lib");

            // The Rust compiler will look for libfoo.a and foo.lib, but the
            // MSVC linker will also be passed foo.lib, so be sure that both
            // exist for now.
            let lib_dst = dst.join(format!("{}.lib", lib_name));
            let _ = fs::remove_file(&lib_dst);
            fs::hard_link(dst.join(output), lib_dst).unwrap();
        } else {
            run(Command::new(&ar(&target)).arg("crus")
                                          .arg(&dst.join(output))
                                          .args(&objects)
                                          .args(&self.objects),
                &ar(&target));
        }
        println!("cargo:rustc-link-search=native={}", dst.display());
        println!("cargo:rustc-link-lib=static={}",
                 &output[3..output.len() - 2]);

        // Add specific C++ libraries, if enabled.
        if self.cpp {
            if let Some(ref stdlib) = self.cpp_link_stdlib {
                println!("cargo:rustc-link-lib={}", stdlib);
            }
        }
    }

    fn compiler(&self, target: &str) -> String {
        if self.cpp {
            gxx(target)
        } else {
            gcc(target)
        }
    }

    fn compile_flags(&self) -> Vec<String> {
        if self.cpp {
            cxxflags()
        } else {
            cflags()
        }
    }

    fn compile_cmd(&self, target: &str) -> Command {
        let opt_level = getenv_unwrap("OPT_LEVEL");
        let profile = getenv_unwrap("PROFILE");
        println!("{} {}", profile, opt_level);

        let mut cmd = Command::new(self.compiler(&target));

        if target.contains("msvc") {
            cmd.arg("/c");
            cmd.arg("/MD"); // link against msvcrt.dll for now
            cmd.arg(format!("/O{}", opt_level));
        } else {
            cmd.arg(format!("-O{}", opt_level));
            cmd.arg("-c");
            cmd.arg("-ffunction-sections").arg("-fdata-sections");
        }
        cmd.args(&self.compile_flags());

        if target.contains("-ios") {
            cmd.args(&ios_flags(&target));
        } else if !target.contains("msvc") {
            if target.contains("windows") {
                cmd.arg("-mwin32");
            }

            if target.contains("i686") {
                cmd.arg("-m32");
            } else if target.contains("x86_64") {
                cmd.arg("-m64");
            }

            if !target.contains("i686") {
                cmd.arg("-fPIC");
            }
        }

        if self.cpp && !target.contains("msvc") {
            if let Some(ref stdlib) = self.cpp_set_stdlib {
                cmd.arg(&format!("-stdlib=lib{}", stdlib));
            }
        }

        for directory in self.include_directories.iter() {
            cmd.arg(if target.contains("msvc") {"/I"} else {"-I"});
            cmd.arg(directory);
        }

        for flag in self.flags.iter() {
            cmd.arg(flag);
        }

        for &(ref key, ref value) in self.definitions.iter() {
            let lead = if target.contains("msvc") {"/"} else {"-"};
            if let &Some(ref value) = value {
                cmd.arg(&format!("{}D{}={}", lead, key, value));
            } else {
                cmd.arg(&format!("{}D{}", lead, key));
            }
        }
        return cmd;
    }
}

fn run(cmd: &mut Command, program: &str) {
    println!("running: {:?}", cmd);
    let status = match cmd.status() {
        Ok(status) => status,
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            let extra = if cfg!(windows) {
                " (see https://github.com/alexcrichton/gcc-rs#windows-notes \
                   for help)"
            } else {
                ""
            };
            fail(&format!("failed to execute command: {}\nIs `{}` \
                           not installed?{}", e, program, extra));
        }
        Err(e) => fail(&format!("failed to execute command: {}", e)),
    };
    if !status.success() {
        fail(&format!("command did not execute successfully, got: {}", status));
    }
}

fn get_var(var_base: &str) -> Result<String, String> {
    let target = getenv_unwrap("TARGET");
    let host = getenv_unwrap("HOST");
    let kind = if host == target {"HOST"} else {"TARGET"};
    let target_u = target.replace("-", "_");
    let res = getenv(&format!("{}_{}", var_base, target))
        .or_else(|| getenv(&format!("{}_{}", var_base, target_u)))
        .or_else(|| getenv(&format!("{}_{}", kind, var_base)))
        .or_else(|| getenv(var_base));

    match res {
        Some(res) => Ok(res),
        None => Err("Could not get environment variable".to_string()),
    }
}

fn gcc(target: &str) -> String {
    let is_android = target.find("android").is_some();

    get_var("CC").unwrap_or(if cfg!(windows) {
        if target.contains("msvc") {
            "cl".to_string()
        } else {
            "gcc".to_string()
        }
    } else if is_android {
        format!("{}-gcc", target)
    } else {
        "cc".to_string()
    })
}

fn gxx(target: &str) -> String {
    let is_android = target.find("android").is_some();

    get_var("CXX").unwrap_or(if cfg!(windows) {
        if target.contains("msvc") {
            "cl".to_string()
        } else {
            "g++".to_string()
        }
    } else if is_android {
        format!("{}-g++", target)
    } else {
        "c++".to_string()
    })
}

fn ar(target: &str) -> String {
    let is_android = target.find("android").is_some();

    get_var("AR").unwrap_or(if is_android {
        format!("{}-ar", target)
    } else {
        "ar".to_string()
    })
}

fn envflags(name: &str) -> Vec<String> {
    get_var(name).unwrap_or(String::new())
       .split(|c: char| c.is_whitespace()).filter(|s| !s.is_empty())
       .map(|s| s.to_string())
       .collect()
}

fn cflags() -> Vec<String> {
    envflags("CFLAGS")
}

fn cxxflags() -> Vec<String> {
    envflags("CXXFLAGS")
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
        "arm64" | "aarch64" => ArchSpec::Device("arm64"),
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
        .stderr(Stdio::inherit())
        .output()
        .unwrap()
        .stdout;

    let sdk_path = String::from_utf8(sdk_path).unwrap();

    res.push("-isysroot".to_string());
    res.push(sdk_path.trim().to_string());

    res
}

fn fail(s: &str) -> ! {
    println!("\n\n{}\n\n", s);
    panic!()
}
