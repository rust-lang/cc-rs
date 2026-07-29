#![allow(dead_code)]
#![allow(clippy::disallowed_methods)]

mod global_env;

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use tempfile::{Builder, TempDir};

pub use self::global_env::GlobalEnv;

pub struct Test {
    pub td: TempDir,
    pub gcc: PathBuf,
    pub msvc: bool,
    pub msvc_autodetect: bool,
    pub env: GlobalEnv,
}

pub struct Execution {
    pub args: Vec<String>,
}

impl Test {
    #[track_caller]
    pub fn new() -> Test {
        let mut env = GlobalEnv::lock();

        // This is ugly: `sccache` needs to introspect the compiler it is
        // executing, as it adjusts its behavior depending on the
        // language/compiler. This crate's test driver uses mock compilers that
        // are obviously not supported by sccache, so the tests fail if
        // RUSTC_WRAPPER is set. rust doesn't build test dependencies with
        // the `test` feature enabled, so we can't conditionally disable the
        // usage of `sccache` if running in a test environment, at least not
        // without setting an environment variable here and testing for it
        // there. Explicitly deasserting RUSTC_WRAPPER here seems to be the
        // lesser of the two evils.
        env.remove("RUSTC_WRAPPER");

        // cc-rs prefers these env vars to the wrappers. We set these in some tests, so unset them so the wrappers get used
        env.remove("CC");
        env.remove("CXX");
        env.remove("AR");

        // Some tests check that a flag is *not* present.  These tests might fail if the flag is set in the
        // CFLAGS or CXXFLAGS environment variables.  This clears the CFLAGS and CXXFLAGS
        // variables to make sure that the tests can run correctly.
        env.set("CFLAGS", "");
        env.set("CXXFLAGS", "");

        let mut gcc = env::current_exe().unwrap();
        gcc.pop();
        if gcc.ends_with("deps") {
            gcc.pop();
        }
        let td = Builder::new()
            .prefix("cc-shim-test")
            .tempdir_in(&gcc)
            .unwrap();
        gcc.push(format!("cc-shim{}", env::consts::EXE_SUFFIX));

        Test {
            td,
            gcc,
            msvc: false,
            msvc_autodetect: false,
            env,
        }
    }

    #[track_caller]
    pub fn gnu() -> Test {
        let t = Test::new();
        t.shim("cc").shim("c++").shim("ar");
        t
    }

    #[track_caller]
    pub fn msvc() -> Test {
        let mut t = Test::new();
        t.shim("cl").shim("lib.exe");
        t.msvc = true;
        t
    }

    // For msvc_autodetect, don't explicitly set the compiler - let the build system discover it
    #[track_caller]
    pub fn msvc_autodetect() -> Test {
        let mut t = Test::new();
        t.shim("cl").shim("clang-cl.exe").shim("lib.exe");
        t.msvc_autodetect = true;
        t
    }

    #[track_caller]
    pub fn clang() -> Test {
        let t = Test::new();
        t.shim("clang").shim("clang++").shim("ar");
        t
    }

    pub fn shim(&self, name: &str) -> &Test {
        let name = if name.ends_with(env::consts::EXE_SUFFIX) {
            name.to_string()
        } else {
            format!("{}{}", name, env::consts::EXE_SUFFIX)
        };
        link_or_copy(&self.gcc, self.td.path().join(name)).unwrap();
        self
    }

    pub fn gcc(&self) -> cc::Build {
        let mut cfg = cc::Build::new();
        let target = if self.msvc || self.msvc_autodetect {
            "x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            "x86_64-apple-darwin"
        } else {
            "x86_64-unknown-linux-gnu"
        };

        cfg.target(target)
            .host(target)
            .opt_level(2)
            .debug(false)
            .out_dir(self.td.path())
            .env("PATH", self.path())
            .env("CC_SHIM_OUT_DIR", self.td.path());
        if self.msvc {
            cfg.compiler(self.td.path().join("cl"));
            cfg.archiver(self.td.path().join("lib.exe"));
        }
        cfg
    }

    fn path(&self) -> OsString {
        let mut path = env::split_paths(&env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        path.insert(0, self.td.path().to_owned());
        env::join_paths(path).unwrap()
    }

    pub fn cmd(&self, i: u32) -> Execution {
        let mut s = String::new();
        File::open(self.td.path().join(format!("out{}", i)))
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        Execution {
            args: s.lines().map(|s| s.to_string()).collect(),
        }
    }
}

impl Execution {
    #[track_caller]
    pub fn must_have<P: AsRef<OsStr>>(&self, p: P) -> &Execution {
        if !self.has(p.as_ref()) {
            panic!("didn't find {:?} in {:?}", p.as_ref(), self.args);
        } else {
            self
        }
    }

    #[track_caller]
    pub fn must_not_have<P: AsRef<OsStr>>(&self, p: P) -> &Execution {
        if self.has(p.as_ref()) {
            panic!("found {:?}", p.as_ref());
        } else {
            self
        }
    }

    pub fn has(&self, p: &OsStr) -> bool {
        self.args.iter().any(|arg| OsStr::new(arg) == p)
    }

    #[track_caller]
    pub fn must_have_in_order(&self, before: &str, after: &str) -> &Execution {
        let before_position = self
            .args
            .iter()
            .rposition(|x| OsStr::new(x) == OsStr::new(before));
        let after_position = self
            .args
            .iter()
            .rposition(|x| OsStr::new(x) == OsStr::new(after));
        match (before_position, after_position) {
            (Some(b), Some(a)) if b < a => {}
            (b, a) => panic!(
                "{:?} (last position: {:?}) did not appear before {:?} (last position: {:?}): {:?}",
                before, b, after, a, self.args
            ),
        };
        self
    }
}

/// Hard link an executable or copy it if that fails.
///
/// We first try to hard link an executable to save space. If that fails (as on Windows with
/// different mount points, issue #60), we copy.
#[cfg(not(target_os = "macos"))]
fn link_or_copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::hard_link(from, to).or_else(|_| fs::copy(from, to).map(|_| ()))
}

/// Copy an executable.
///
/// On macOS, hard linking the executable leads to strange failures (issue #419), so we just copy.
#[cfg(target_os = "macos")]
fn link_or_copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    fs::copy(from, to).map(|_| ())
}
