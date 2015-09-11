//! A library for build scripts to compile custom C code
//!
//! This library is intended to be used as a `build-dependencies` entry in
//! `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! gcc = "0.3"
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
#![deny(missing_docs)]

use std::env;
use std::ffi::{OsString, OsStr};
use std::fs;
use std::io;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};

#[cfg(windows)]
mod registry;
pub mod windows_registry;

/// Extra configuration to pass to gcc.
pub struct Config {
    include_directories: Vec<PathBuf>,
    definitions: Vec<(String, Option<String>)>,
    objects: Vec<PathBuf>,
    flags: Vec<String>,
    files: Vec<PathBuf>,
    cpp: bool,
    cpp_link_stdlib: Option<Option<String>>,
    cpp_set_stdlib: Option<String>,
    target: Option<String>,
    host: Option<String>,
    out_dir: Option<PathBuf>,
    opt_level: Option<u32>,
    debug: Option<bool>,
    env: Vec<(OsString, OsString)>,
    compiler: Option<PathBuf>,
    archiver: Option<PathBuf>,
    cargo_metadata: bool,
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
            cpp_link_stdlib: None,
            cpp_set_stdlib: None,
            target: None,
            host: None,
            out_dir: None,
            opt_level: None,
            debug: None,
            env: Vec::new(),
            compiler: None,
            archiver: None,
            cargo_metadata: true
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
    pub fn cpp_link_stdlib(&mut self, cpp_link_stdlib: Option<&str>)
                           -> &mut Config {
        self.cpp_link_stdlib = Some(cpp_link_stdlib.map(|s| s.into()));
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
    pub fn cpp_set_stdlib(&mut self, cpp_set_stdlib: Option<&str>)
                          -> &mut Config {
        self.cpp_set_stdlib = cpp_set_stdlib.map(|s| s.into());
        self.cpp_link_stdlib(cpp_set_stdlib);
        self
    }

    /// Configures the target this configuration will be compiling for.
    ///
    /// This option is automatically scraped from the `TARGET` environment
    /// variable by build scripts, so it's not required to call this function.
    pub fn target(&mut self, target: &str) -> &mut Config {
        self.target = Some(target.to_string());
        self
    }

    /// Configures the host assumed by this configuration.
    ///
    /// This option is automatically scraped from the `HOST` environment
    /// variable by build scripts, so it's not required to call this function.
    pub fn host(&mut self, host: &str) -> &mut Config {
        self.host = Some(host.to_string());
        self
    }

    /// Configures the optimization level of the generated object files.
    ///
    /// This option is automatically scraped from the `OPT_LEVEL` environment
    /// variable by build scripts, so it's not required to call this function.
    pub fn opt_level(&mut self, opt_level: u32) -> &mut Config {
        self.opt_level = Some(opt_level);
        self
    }

    /// Configures whether the compiler will emit debug information when
    /// generating object files.
    ///
    /// This option is automatically scraped from the `PROFILE` environment
    /// variable by build scripts (only enabled when the profile is "debug"), so
    /// it's not required to call this function.
    pub fn debug(&mut self, debug: bool) -> &mut Config {
        self.debug = Some(debug);
        self
    }

    /// Configures the output directory where all object files and static
    /// libraries will be located.
    ///
    /// This option is automatically scraped from the `OUT_DIR` environment
    /// variable by build scripts, so it's not required to call this function.
    pub fn out_dir<P: AsRef<Path>>(&mut self, out_dir: P) -> &mut Config {
        self.out_dir = Some(out_dir.as_ref().to_owned());
        self
    }

    /// Configures the compiler to be used to produce output.
    ///
    /// This option is automatically determined from the target platform or a
    /// number of environment variables, so it's not required to call this
    /// function.
    pub fn compiler<P: AsRef<Path>>(&mut self, compiler: P) -> &mut Config {
        self.compiler = Some(compiler.as_ref().to_owned());
        self
    }

    /// Configures the tool used to assemble archives.
    ///
    /// This option is automatically determined from the target platform or a
    /// number of environment variables, so it's not required to call this
    /// function.
    pub fn archiver<P: AsRef<Path>>(&mut self, archiver: P) -> &mut Config {
        self.archiver = Some(archiver.as_ref().to_owned());
        self
    }
    /// Define whether metadata should be emitted for cargo allowing it to
    /// automatically link the binary. Defaults to `true`.
    pub fn cargo_metadata(&mut self, cargo_metadata: bool) -> &mut Config {
        self.cargo_metadata = cargo_metadata;
        self
    }


    #[doc(hidden)]
    pub fn __set_env<A, B>(&mut self, a: A, b: B) -> &mut Config
        where A: AsRef<OsStr>, B: AsRef<OsStr>
    {
        self.env.push((a.as_ref().to_owned(), b.as_ref().to_owned()));
        self
    }

    /// Run the compiler, generating the file `output`
    ///
    /// The name `output` must begin with `lib` and end with `.a`
    pub fn compile(&self, output: &str) {
        assert!(output.starts_with("lib"));
        assert!(output.ends_with(".a"));
        let lib_name = &output[3..output.len() - 2];
        let dst = self.get_out_dir();

        let mut objects = Vec::new();
        for file in self.files.iter() {
            let obj = dst.join(file).with_extension("o");
            self.compile_object(file, &obj);
            objects.push(obj);
        }

        self.assemble(lib_name, &dst.join(output), &objects);

        if self.cargo_metadata {
            println!("cargo:rustc-link-lib=static={}",
                     &output[3..output.len() - 2]);
            println!("cargo:rustc-link-search=native={}", dst.display());

            // Add specific C++ libraries, if enabled.
            if self.cpp {
                if let Some(stdlib) = self.get_cpp_link_stdlib() {
                    println!("cargo:rustc-link-lib={}", stdlib);
                }
            }
        }
    }

    fn compile_object(&self, file: &Path, dst: &Path) {
        let is_asm = file.extension().and_then(|s| s.to_str()) == Some("asm");
        let msvc = self.get_target().contains("msvc");
        let (mut cmd, name) = if msvc && is_asm {
            self.msvc_macro_assembler()
        } else {
            self.compile_cmd()
        };
        if msvc {
            cmd.arg("/nologo");
        }
        fs::create_dir_all(&dst.parent().unwrap()).unwrap();
        if msvc && is_asm {
            cmd.arg("/Fo").arg(dst);
        } else if msvc {
            let mut s = OsString::from("/Fo:");
            s.push(&dst);
            cmd.arg(s);
        } else {
            cmd.arg("-o").arg(&dst);
        }
        if msvc {
            cmd.arg("/c");
        }
        cmd.arg(file);

        run(&mut cmd, &name);
    }

    fn compile_cmd(&self) -> (Command, String) {
        let opt_level = self.get_opt_level();
        let debug = self.get_debug();
        let target = self.get_target();
        let msvc = target.contains("msvc");
        println!("debug={} opt-level={}", debug, opt_level);

        let (mut cmd, name) = self.get_compiler();

        if msvc {
            cmd.arg("/MD"); // link against msvcrt.dll for now
            if opt_level != 0 {
                cmd.arg("/O2");
            }
        } else {
            cmd.arg(format!("-O{}", opt_level));
            cmd.arg("-c");
            cmd.arg("-ffunction-sections").arg("-fdata-sections");
        }
        cmd.args(&self.envflags(if self.cpp {"CXXFLAGS"} else {"CFLAGS"}));

        if debug {
            cmd.arg(if msvc {"/Z7"} else {"-g"});
        }

        if target.contains("-ios") {
            self.ios_flags(&mut cmd);
        } else if !msvc {
            if target.contains("i686") {
                cmd.arg("-m32");
            } else if target.contains("x86_64") {
                cmd.arg("-m64");
            }

            if !target.contains("i686") && !target.contains("windows-gnu") {
                cmd.arg("-fPIC");
            }
        }

        if self.cpp && !msvc {
            if let Some(ref stdlib) = self.cpp_set_stdlib {
                cmd.arg(&format!("-stdlib=lib{}", stdlib));
            }
        }

        for directory in self.include_directories.iter() {
            cmd.arg(if msvc {"/I"} else {"-I"});
            cmd.arg(directory);
        }

        for flag in self.flags.iter() {
            cmd.arg(flag);
        }

        for &(ref key, ref value) in self.definitions.iter() {
            let lead = if msvc {"/"} else {"-"};
            if let &Some(ref value) = value {
                cmd.arg(&format!("{}D{}={}", lead, key, value));
            } else {
                cmd.arg(&format!("{}D{}", lead, key));
            }
        }
        (cmd, name)
    }

    fn msvc_macro_assembler(&self) -> (Command, String) {
        let target = self.get_target();
        let tool = if target.contains("x86_64") {"ml64.exe"} else {"ml.exe"};
        let mut cmd = windows_registry::find(&target, tool).unwrap_or_else(|| {
            self.cmd(tool)
        });
        for directory in self.include_directories.iter() {
            cmd.arg("/I").arg(directory);
        }
        for &(ref key, ref value) in self.definitions.iter() {
            if let &Some(ref value) = value {
                cmd.arg(&format!("/D{}={}", key, value));
            } else {
                cmd.arg(&format!("/D{}", key));
            }
        }
        (cmd, tool.to_string())
    }

    fn assemble(&self, lib_name: &str, dst: &Path, objects: &[PathBuf]) {
        let target = self.get_target();
        if target.contains("msvc") {
            let mut cmd = match self.archiver {
                Some(ref s) => self.cmd(s),
                None => windows_registry::find(&target, "lib.exe")
                                         .unwrap_or(self.cmd("lib.exe")),
            };
            let mut out = OsString::from("/OUT:");
            out.push(dst);
            run(cmd.arg(out).arg("/nologo")
                   .args(objects)
                   .args(&self.objects), "lib.exe");

            // The Rust compiler will look for libfoo.a and foo.lib, but the
            // MSVC linker will also be passed foo.lib, so be sure that both
            // exist for now.
            let lib_dst = dst.with_file_name(format!("{}.lib", lib_name));
            let _ = fs::remove_file(&lib_dst);
            fs::hard_link(dst, lib_dst).unwrap();
        } else {
            let ar = self.get_ar();
            let cmd = ar.file_name().unwrap().to_string_lossy();
            run(self.cmd(&ar).arg("crus")
                                 .arg(dst)
                                 .args(objects)
                                 .args(&self.objects), &cmd);
        }
    }

    fn ios_flags(&self, cmd: &mut Command) {
        enum ArchSpec {
            Device(&'static str),
            Simulator(&'static str),
        }

        let target = self.get_target();
        let arch = target.split('-').nth(0).unwrap();
        let arch = match arch {
            "arm" | "armv7" | "thumbv7" => ArchSpec::Device("armv7"),
            "armv7s" | "thumbv7s" => ArchSpec::Device("armv7s"),
            "arm64" | "aarch64" => ArchSpec::Device("arm64"),
            "i386" | "i686" => ArchSpec::Simulator("-m32"),
            "x86_64" => ArchSpec::Simulator("-m64"),
            _ => fail("Unknown arch for iOS target")
        };

        let sdk = match arch {
            ArchSpec::Device(arch) => {
                cmd.arg("-arch").arg(arch);
                "iphoneos"
            },
            ArchSpec::Simulator(arch) => {
                cmd.arg(arch);
                "iphonesimulator"
            }
        };

        println!("Detecting iOS SDK path for {}", sdk);
        let sdk_path = self.cmd("xcrun")
            .arg("--show-sdk-path")
            .arg("--sdk")
            .arg(sdk)
            .stderr(Stdio::inherit())
            .output()
            .unwrap()
            .stdout;

        let sdk_path = String::from_utf8(sdk_path).unwrap();

        cmd.arg("-isysroot");
        cmd.arg(sdk_path.trim());
    }

    fn cmd<P: AsRef<OsStr>>(&self, prog: P) -> Command {
        let mut cmd = Command::new(prog);
        for &(ref a, ref b) in self.env.iter() {
            cmd.env(a, b);
        }
        return cmd
    }

    fn get_compiler(&self) -> (Command, String) {
        if let Some(ref c) = self.compiler {
            return (self.cmd(c), c.file_name().unwrap()
                                  .to_string_lossy().into_owned())
        }
        let target = self.get_target();
        let (env, msvc, gnu, default) = if self.cpp {
            ("CXX", "cl", "g++", "c++")
        } else {
            ("CC", "cl", "gcc", "cc")
        };
        self.get_var(env).ok().map(|env| {
            let fname = Path::new(&env).file_name().unwrap().to_string_lossy()
                                       .into_owned();
            (self.cmd(env), fname)
        }).or_else(|| {
            windows_registry::find(&target, "cl.exe").map(|cmd| {
                (cmd, "cl.exe".to_string())
            })
        }).unwrap_or_else(|| {
            let compiler = if target.contains("windows") {
                if target.contains("msvc") {
                    msvc.to_string()
                } else {
                    gnu.to_string()
                }
            } else if target.contains("android") {
                format!("{}-{}", target, gnu)
            } else {
                default.to_string()
            };
            (self.cmd(compiler.clone()), compiler)
        })
    }

    fn get_var(&self, var_base: &str) -> Result<String, String> {
        let target = self.get_target();
        let host = self.get_host();
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

    fn envflags(&self, name: &str) -> Vec<String> {
        self.get_var(name).unwrap_or(String::new())
            .split(|c: char| c.is_whitespace()).filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Returns the default C++ standard library for the current target: `libc++`
    /// for OS X and `libstdc++` for anything else.
    fn get_cpp_link_stdlib(&self) -> Option<String> {
        self.cpp_link_stdlib.clone().unwrap_or_else(|| {
            let target = self.get_target();
            if target.contains("msvc") {
                None
            } else if target.contains("darwin") {
                Some("c++".to_string())
            } else {
                Some("stdc++".to_string())
            }
        })
    }

    fn get_ar(&self) -> PathBuf {
        self.archiver.clone().or_else(|| {
            self.get_var("AR").map(PathBuf::from).ok()
        }).unwrap_or_else(|| {
            if self.get_target().contains("android") {
                PathBuf::from(format!("{}-ar", self.get_target()))
            } else {
                PathBuf::from("ar")
            }
        })
    }

    fn get_target(&self) -> String {
        self.target.clone().unwrap_or_else(|| getenv_unwrap("TARGET"))
    }

    fn get_host(&self) -> String {
        self.host.clone().unwrap_or_else(|| getenv_unwrap("HOST"))
    }

    fn get_opt_level(&self) -> u32 {
        self.opt_level.unwrap_or_else(|| {
            getenv_unwrap("OPT_LEVEL").parse().unwrap()
        })
    }

    fn get_debug(&self) -> bool {
        self.debug.unwrap_or_else(|| getenv_unwrap("PROFILE") == "debug")
    }

    fn get_out_dir(&self) -> PathBuf {
        self.out_dir.clone().unwrap_or_else(|| {
            env::var_os("OUT_DIR").map(PathBuf::from).unwrap()
        })
    }
}

fn run(cmd: &mut Command, program: &str) {
    println!("running: {:?}", cmd);
    let status = match cmd.status() {
        Ok(status) => status,
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            let extra = if cfg!(windows) {
                " (see https://github.com/alexcrichton/gcc-rs#compile-time-requirements \
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

fn fail(s: &str) -> ! {
    println!("\n\n{}\n\n", s);
    panic!()
}
