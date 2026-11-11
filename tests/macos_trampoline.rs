//! On macOS the compilers in `/usr/bin` are trampolines that ask xcrun which
//! compiler to run on every call. cc asks once and runs that compiler, passing
//! it the SDK the way it does for any Xcode toolchain compiler.
#![cfg(target_os = "macos")]
#![allow(clippy::disallowed_methods)]

use std::ffi::{OsStr, OsString};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use crate::support::Test;

mod support;

type Env = Vec<(OsString, OsString)>;

fn host() -> String {
    format!("{}-apple-darwin", std::env::consts::ARCH)
}

/// xcrun's trimmed answer in `env`, or `None` if it fails.
fn xcrun(env: &Env, args: &[&str]) -> Option<String> {
    let output = Command::new("xcrun")
        .args(args)
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k, v)))
        .output()
        .unwrap();
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).unwrap().trim().to_owned())
}

/// `test`'s environment without `SDKROOT`, or `None` when its `PATH` doesn't
/// find Apple's `/usr/bin/<name>` first.
fn trampoline_env(test: &Test, name: &str) -> Option<Env> {
    let env: Env = test
        .env
        .iter()
        .filter(|(k, _)| *k != "SDKROOT")
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let (_, path) = env.iter().find(|(k, _)| k == "PATH")?;
    let found = std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|tool| tool.is_file())?;
    (found == Path::new("/usr/bin").join(name)).then_some(env)
}

fn new_build(test: &Test, target: &str) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .target(target)
        .host(&host())
        .opt_level(0)
        .debug(false)
        .cargo_metadata(false)
        .out_dir(test.td.path());
    build
}

/// The compiler a clone of `build` gets in `env`. Clones share the build's
/// cache.
fn compiler(build: &cc::Build, env: &Env) -> cc::Tool {
    let mut build = build.clone();
    build.set_envs_snapshot(env.iter().map(|(k, v)| (k.as_os_str(), v.as_os_str())));
    build.try_get_compiler().unwrap()
}

/// Whether `tool`, run the way cc runs it, compiles a file that includes
/// `<stdio.h>`, which it can't without an SDK.
fn compiles_with_stdio(test: &Test, tool: &cc::Tool) -> bool {
    let src = test.td.path().join("stdio.c");
    std::fs::write(
        &src,
        "#include <stdio.h>\nint main(void) { return puts(\"\"); }\n",
    )
    .unwrap();
    tool.to_command()
        .arg("-c")
        .arg(&src)
        .arg("-o")
        .arg(test.td.path().join("stdio.o"))
        .status()
        .unwrap()
        .success()
}

#[test]
fn runs_the_compiler_xcrun_finds() {
    let test = Test::new();
    let Some(env) = trampoline_env(&test, "cc") else {
        return;
    };
    let tool = compiler(&new_build(&test, &host()), &env);
    assert_eq!(
        tool.path(),
        Path::new(&xcrun(&env, &["--find", "cc"]).unwrap())
    );
    let sdk = xcrun(&env, &["--show-sdk-path", "--sdk", "macosx"]).unwrap();
    let sysroot = tool
        .args()
        .iter()
        .position(|arg| arg == "-isysroot")
        .map(|i| tool.args()[i + 1].as_os_str());
    assert_eq!(sysroot, Some(OsStr::new(&sdk)));
    assert!(compiles_with_stdio(&test, &tool));
}

#[test]
fn runs_the_compiler_xcrun_finds_for_ios() {
    let test = Test::new();
    let Some(env) = trampoline_env(&test, "clang") else {
        return;
    };
    if xcrun(&env, &["--show-sdk-path", "--sdk", "iphoneos"]).is_none() {
        return;
    }
    let tool = compiler(&new_build(&test, "aarch64-apple-ios"), &env);
    assert_eq!(
        tool.path(),
        Path::new(&xcrun(&env, &["--find", "clang"]).unwrap())
    );
    assert!(compiles_with_stdio(&test, &tool));
}

#[test]
fn keeps_a_cc_found_earlier_in_path() {
    let test = Test::new();
    let Some(env) = trampoline_env(&test, "cc") else {
        return;
    };
    // Like a ccache or Xcode directory ahead of `/usr/bin` in `PATH`.
    let dir = test.td.path().join("bin");
    std::fs::create_dir(&dir).unwrap();
    symlink("/usr/bin/cc", dir.join("cc")).unwrap();
    let shadowed: Env = env
        .iter()
        .map(|(k, v)| {
            if k != "PATH" {
                return (k.clone(), v.clone());
            }
            let mut path = dir.clone().into_os_string();
            path.push(":");
            path.push(v);
            (k.clone(), path)
        })
        .collect();
    // One build for both, so the answer cached for `env` mustn't be used for
    // `shadowed`.
    let build = new_build(&test, &host());
    assert_ne!(compiler(&build, &env).path(), Path::new("cc"));
    assert_eq!(compiler(&build, &shadowed).path(), Path::new("cc"));
}
