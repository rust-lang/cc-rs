extern crate gcc;
extern crate tempdir;

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

use tempdir::TempDir;

struct Test {
    td: TempDir,
    gcc: PathBuf,
    msvc: bool,
}

struct Execution {
    args: Vec<String>,
}

impl Test {
    fn new() -> Test {
        let mut gcc = PathBuf::from(env::current_exe().unwrap());
        gcc.pop();
        gcc.push(format!("gcc-shim{}", env::consts::EXE_SUFFIX));
        Test {
            td: TempDir::new("gcc-test").unwrap(),
            gcc: gcc,
            msvc: false,
        }
    }

    fn gnu() -> Test {
        let t = Test::new();
        t.shim("cc").shim("ar");
        return t
    }

    fn msvc() -> Test {
        let mut t = Test::new();
        t.shim("cl").shim("lib.exe");
        t.msvc = true;
        return t
    }

    fn shim(&self, name: &str) -> &Test {
        let fname = format!("{}{}", name, env::consts::EXE_SUFFIX);
        fs::hard_link(&self.gcc, self.td.path().join(fname)).unwrap();
        self
    }

    fn gcc(&self) -> gcc::Config {
        let mut cfg = gcc::Config::new();
        let mut path = env::split_paths(&env::var_os("PATH").unwrap())
                           .collect::<Vec<_>>();
        path.insert(0, self.td.path().to_owned());
        let target = if self.msvc {
            "x86_64-pc-windows-msvc"
        } else {
            "x86_64-unknown-linux-gnu"
        };

        cfg.target(target).host(target)
           .opt_level(2)
           .debug(false)
           .out_dir(self.td.path())
           .__set_env("PATH", env::join_paths(path).unwrap())
           .__set_env("GCCTEST_OUT_DIR", self.td.path());
        if self.msvc {
            cfg.compiler(self.td.path().join("cl"));
            cfg.archiver(self.td.path().join("lib.exe"));
        }
        return cfg
    }

    fn cmd(&self, i: u32) -> Execution {
        let mut s = String::new();
        File::open(self.td.path().join(format!("out{}", i))).unwrap()
             .read_to_string(&mut s).unwrap();
        Execution {
            args: s.lines().map(|s| s.to_string()).collect(),
        }
    }
}

impl Execution {
    fn must_have<P: AsRef<OsStr>>(&self, p: P) -> &Execution {
        if !self.has(p.as_ref()) {
            panic!("didn't find {:?} in {:?}", p.as_ref(), self.args);
        } else {
            self
        }
    }

    fn must_not_have<P: AsRef<OsStr>>(&self, p: P) -> &Execution {
        if self.has(p.as_ref()) {
            panic!("found {:?}", p.as_ref());
        } else {
            self
        }
    }

    fn has(&self, p: &OsStr) -> bool {
        self.args.iter().any(|arg| {
            OsStr::new(arg) == p
        })
    }
}

#[test]
fn gnu_smoke() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("-O2")
               .must_have("foo.c")
               .must_not_have("-g")
               .must_have("-c")
               .must_have("-ffunction-sections")
               .must_have("-fdata-sections");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

#[test]
fn gnu_opt_level_1() {
    let test = Test::gnu();
    test.gcc()
        .opt_level(1)
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("-O1")
               .must_not_have("-O2");
}

#[test]
fn gnu_debug() {
    let test = Test::gnu();
    test.gcc()
        .debug(true)
        .file("foo.c").compile("libfoo.a");
    test.cmd(0).must_have("-g");
}

#[test]
fn gnu_x86_64() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("x86_64-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c").compile("libfoo.a");

        test.cmd(0).must_have("-fPIC")
                   .must_have("-m64");
    }
}

#[test]
fn gnu_i686() {
    for vendor in &["unknown-linux-gnu", "apple-darwin"] {
        let target = format!("i686-{}", vendor);
        let test = Test::gnu();
        test.gcc()
            .target(&target)
            .host(&target)
            .file("foo.c").compile("libfoo.a");

        test.cmd(0).must_not_have("-fPIC")
                   .must_have("-m32");
    }
}

#[test]
fn gnu_set_stdlib() {
    let test = Test::gnu();
    test.gcc()
        .cpp_set_stdlib(Some("foo"))
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_not_have("-stdlib=foo");
}

#[test]
fn gnu_include() {
    let test = Test::gnu();
    test.gcc()
        .include("foo/bar")
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("-I").must_have("foo/bar");
}

#[test]
fn gnu_define() {
    let test = Test::gnu();
    test.gcc()
        .define("FOO", Some("bar"))
        .define("BAR", None)
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("-DFOO=bar").must_have("-DBAR");
}

#[test]
fn gnu_compile_assembly() {
    let test = Test::gnu();
    test.gcc()
        .file("foo.S").compile("libfoo.a");
    test.cmd(0).must_have("foo.S");
}

#[test]
fn msvc_smoke() {
    let test = Test::msvc();
    test.gcc()
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("/O2")
               .must_have("foo.c")
               .must_not_have("/Z7")
               .must_have("/c");
    test.cmd(1).must_have(test.td.path().join("foo.o"));
}

#[test]
fn msvc_opt_level_0() {
    let test = Test::msvc();
    test.gcc()
        .opt_level(0)
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_not_have("/O2");
}

#[test]
fn msvc_debug() {
    let test = Test::msvc();
    test.gcc()
        .debug(true)
        .file("foo.c").compile("libfoo.a");
    test.cmd(0).must_have("/Z7");
}

#[test]
fn msvc_include() {
    let test = Test::msvc();
    test.gcc()
        .include("foo/bar")
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("/I").must_have("foo/bar");
}

#[test]
fn msvc_define() {
    let test = Test::msvc();
    test.gcc()
        .define("FOO", Some("bar"))
        .define("BAR", None)
        .file("foo.c").compile("libfoo.a");

    test.cmd(0).must_have("/DFOO=bar").must_have("/DBAR");
}
