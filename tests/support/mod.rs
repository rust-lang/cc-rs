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
    /// The environment [`Test::gcc`] hands to cc with
    /// `Build::set_envs_snapshot`, a copy of the process environment taken by
    /// [`Test::new`] without [`CLEARED_VARS`]. Set cc's own variables such as
    /// `CC` or `CFLAGS` here, before making the build, so the process
    /// environment stays unchanged.
    pub env: EnvsSnapshot,
    /// The process environment, locked while the test runs. cc reads the
    /// variables Cargo sets, such as `OUT_DIR`, from it, and the builds of
    /// [`Test::gcc_with_process_env`] read all of it.
    pub process_env: GlobalEnv,
    family_detection_probes: bool,
    flag_supported_probes: bool,
    ar_detection_probes: bool,
    cpp_stdlib_probes: bool,
}

/// Environment variables for [`Test::gcc`] to hand to cc.
pub struct EnvsSnapshot {
    vars: Vec<(OsString, OsString)>,
}

impl EnvsSnapshot {
    pub fn set(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        self.remove(&key);
        self.vars.push((key.as_ref().into(), value.as_ref().into()));
    }

    pub fn remove(&mut self, key: impl AsRef<OsStr>) {
        let key = key.as_ref();
        // Environment variable names are case-insensitive on Windows.
        self.vars.retain(|(k, _)| {
            if cfg!(windows) {
                !k.eq_ignore_ascii_case(key)
            } else {
                k != key
            }
        });
    }

    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.vars.iter().map(|(key, value)| (&**key, &**value))
    }
}

/// Variables a developer's or CI's environment may set that change what the
/// tests see: cc prefers `CC`, `CXX`, `AR` and `CC_MASM_ASM` to the shims,
/// `CC_PREFER_CLANG_CL_OVER_MSVC` changes which compiler cc looks for on MSVC
/// targets, and some tests check that a flag is *not* passed, which `CFLAGS` or
/// `CXXFLAGS` could add.
const CLEARED_VARS: [&str; 7] = [
    "CC",
    "CXX",
    "AR",
    "CC_MASM_ASM",
    "CC_PREFER_CLANG_CL_OVER_MSVC",
    "CFLAGS",
    "CXXFLAGS",
];

/// Files the shim records cc's own probing invocations in, per probe class.
///
/// A build can run a class more than once, so each class gets several slots,
/// filled in the order the probes run.
const FAMILY_DETECTION_PROBES: &str = "family-detection-probe";
const FLAG_SUPPORTED_PROBES: &str = "flag-supported-probe";
const AR_DETECTION_PROBES: &str = "ar-detection-probe";
const CPP_STDLIB_PROBES: &str = "cpp-stdlib-probe";
const PROBE_SLOTS: usize = 4;

pub struct Execution {
    /// The program the shim was invoked as (`argv[0]`).
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Test {
    #[track_caller]
    pub fn new() -> Test {
        let mut process_env = GlobalEnv::lock();

        // This is ugly: `sccache` needs to introspect the compiler it is
        // executing, as it adjusts its behavior depending on the
        // language/compiler. This crate's test driver uses mock compilers that
        // are obviously not supported by sccache, so the tests fail if
        // RUSTC_WRAPPER is set. rust doesn't build test dependencies with
        // the `test` feature enabled, so we can't conditionally disable the
        // usage of `sccache` if running in a test environment, at least not
        // without setting an environment variable here and testing for it
        // there. Explicitly deasserting RUSTC_WRAPPER here seems to be the
        // lesser of the two evils. cc reads it from the process environment,
        // like the other variables Cargo sets.
        process_env.remove("RUSTC_WRAPPER");

        let mut env = EnvsSnapshot {
            vars: env::vars_os().collect(),
        };
        for var in CLEARED_VARS {
            env.remove(var);
        }

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
            process_env,
            family_detection_probes: false,
            flag_supported_probes: false,
            ar_detection_probes: false,
            cpp_stdlib_probes: false,
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
        let mut cfg = self.gcc_without_out_dir();
        cfg.out_dir(self.td.path());
        cfg
    }

    /// Like [`Self::gcc`], but does not set [`cc::Build::out_dir`].
    ///
    /// Flag-support probes must still work when `OUT_DIR` is unset, as in
    /// rustc bootstrap which is not a Cargo build script.
    pub fn gcc_without_out_dir(&self) -> cc::Build {
        let mut cfg = self.build();
        cfg.set_envs_snapshot(self.env.iter());
        cfg
    }

    /// Like [`Self::gcc`], but cc copies the process environment on first use,
    /// as in a build script, instead of getting [`Test::env`]. Calls
    /// [`Test::clear_process_env`] first.
    pub fn gcc_with_process_env(&mut self) -> cc::Build {
        self.clear_process_env();
        let mut cfg = self.build();
        cfg.out_dir(self.td.path());
        cfg
    }

    /// Remove the variables [`Test::new`] removes from [`Test::env`] from the
    /// process environment too, for builds that read it.
    pub fn clear_process_env(&mut self) {
        for var in CLEARED_VARS {
            self.process_env.remove(var);
        }
    }

    fn build(&self) -> cc::Build {
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
        if self.ar_detection_probes {
            cfg.env(
                "CC_SHIM_OUT_FILES_FOR_AR_DETECTION",
                self.probe_out_files(AR_DETECTION_PROBES),
            );
        }
        if self.cpp_stdlib_probes {
            cfg.env(
                "CC_SHIM_OUT_FILES_FOR_CPP_STDLIB_DETECTION",
                self.probe_out_files(CPP_STDLIB_PROBES),
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
    ///
    /// With the `parallel` feature, the compile commands of one build run at
    /// the same time and can be recorded in any order, so a build of several
    /// files reads each compile back with [`Test::cmd_for_source`] instead.
    pub fn cmd(&self, i: usize) -> Execution {
        self.execution(self.probe_slot("out", i))
    }

    /// Read back the one invocation a build performed that has `src` as an
    /// argument, such as the compile command of that source file.
    #[track_caller]
    pub fn cmd_for_source<P: AsRef<OsStr>>(&self, src: P) -> Execution {
        let src = src.as_ref();
        let mut matches = (0..)
            .map(|i| self.probe_slot("out", i))
            .take_while(|path| path.exists())
            .map(|path| self.execution(path))
            .filter(|execution| execution.has(src));
        let Some(execution) = matches.next() else {
            panic!("no recorded invocation has {:?}", src);
        };
        if let Some(other) = matches.next() {
            panic!(
                "more than one recorded invocation has {:?}: {:?} and {:?}",
                src, execution.args, other.args
            );
        }
        execution
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

    /// Record cc's Android `llvm-ar` probe, so
    /// [`Test::get_ar_detection_probes`] can read it back.
    pub fn collect_ar_detection_probes(&mut self) -> &mut Self {
        self.ar_detection_probes = true;
        self
    }

    /// Read back the `i`-th archiver detection probe recorded after
    /// [`Test::collect_ar_detection_probes`].
    pub fn get_ar_detection_probes(&self, i: usize) -> Execution {
        self.execution(self.probe_slot(AR_DETECTION_PROBES, i))
    }

    /// Record cc's C++ standard library probes, so
    /// [`Test::get_cpp_stdlib_probes`] can read them back.
    pub fn collect_cpp_stdlib_probes(&mut self) -> &mut Self {
        self.cpp_stdlib_probes = true;
        self
    }

    /// Read back the `i`-th C++ standard library probe recorded after
    /// [`Test::collect_cpp_stdlib_probes`], or `None` if fewer ran.
    pub fn get_cpp_stdlib_probes(&self, i: usize) -> Option<Execution> {
        let path = self.probe_slot(CPP_STDLIB_PROBES, i);
        path.exists().then(|| self.execution(path))
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
        let mut lines = s.lines().map(|s| s.to_string());
        let Some(program) = lines.next() else {
            panic!("empty recording at {}", path.display());
        };
        Execution {
            program: program.into(),
            args: lines.collect(),
        }
    }
}

impl Execution {
    /// Checks that this invocation ran a program with the file stem `stem`.
    #[track_caller]
    pub fn must_run(&self, stem: &str) -> &Execution {
        if self.program.file_stem() != Some(OsStr::new(stem)) {
            panic!("expected {:?} to run, found {:?}", stem, self.program);
        }
        self
    }

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
