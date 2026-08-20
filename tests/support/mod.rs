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
    family_detection_probes: bool,
    flag_supported_probes: bool,
}

/// Files the shim records cc's own probing invocations in, per probe class.
///
/// A build can run a class more than once, so each class gets several slots,
/// filled in the order the probes run.
const FAMILY_DETECTION_PROBES: &str = "family-detection-probe";
const FLAG_SUPPORTED_PROBES: &str = "flag-supported-probe";
const PROBE_SLOTS: usize = 4;

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

        let td = Builder::new()
            .prefix("cc-shim-test")
            .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .unwrap();

        Test {
            td,
            gcc: env!("CARGO_BIN_EXE_cc-shim").into(),
            msvc: false,
            msvc_autodetect: false,
            env,
            family_detection_probes: false,
            flag_supported_probes: false,
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
        if self.family_detection_probes {
            cfg.env(
                "CC_SHIM_OUT_FILES_FOR_FAMILY_DETECTION",
                self.probe_out_files(FAMILY_DETECTION_PROBES),
            );
        }
        if self.flag_supported_probes {
            cfg.env(
                "CC_SHIM_OUT_FILES_FOR_FLAG_SUPPORT_CHECK",
                self.probe_out_files(FLAG_SUPPORTED_PROBES),
            );
        }
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

    /// Read back the `i`-th invocation a build actually performed.
    ///
    /// cc's own probing invocations are not recorded unless a test asks for
    /// them by name with [`Test::probe_out_files`], so this numbering covers
    /// the compile and archive commands only.
    pub fn cmd(&self, i: usize) -> Execution {
        self.execution(self.probe_slot("out", i))
    }

    /// Record cc's own compiler family detection probes, so
    /// [`Test::get_family_detection_probes`] can read them back. Probe classes a
    /// test does not ask for record nothing and so cannot shift the `out{i}`
    /// numbering [`Test::cmd`] uses.
    pub fn collect_family_detection_probes(&mut self) -> &mut Self {
        self.family_detection_probes = true;
        self
    }

    /// Record cc's own `is_flag_supported` probes, so
    /// [`Test::get_flag_supported_probes`] can read them back.
    pub fn collect_flag_supported_probes(&mut self) -> &mut Self {
        self.flag_supported_probes = true;
        self
    }

    /// Read back the `i`-th family detection probe recorded after
    /// [`Test::collect_family_detection_probes`].
    pub fn get_family_detection_probes(&self, i: usize) -> Execution {
        self.execution(self.probe_slot(FAMILY_DETECTION_PROBES, i))
    }

    /// Read back the `i`-th flag support probe recorded after
    /// [`Test::collect_flag_supported_probes`].
    pub fn get_flag_supported_probes(&self, i: usize) -> Execution {
        self.execution(self.probe_slot(FLAG_SUPPORTED_PROBES, i))
    }

    fn probe_slot(&self, class: &str, i: usize) -> PathBuf {
        self.td.path().join(format!("{class}{i}"))
    }

    fn probe_out_files(&self, class: &str) -> OsString {
        env::join_paths((0..PROBE_SLOTS).map(|i| self.probe_slot(class, i))).unwrap()
    }

    #[track_caller]
    fn execution(&self, path: PathBuf) -> Execution {
        let mut s = String::new();
        File::open(&path)
            .unwrap_or_else(|e| panic!("no recording at {}: {}", path.display(), e))
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
