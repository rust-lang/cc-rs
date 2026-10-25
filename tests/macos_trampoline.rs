//! On macOS the compilers in `/usr/bin` are trampolines that ask xcrun which
//! compiler to run on every call. cc runs that compiler itself, with the
//! environment the trampoline would give it.
#![cfg(target_os = "macos")]
#![allow(clippy::disallowed_methods)]

use std::ffi::{OsStr, OsString};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use crate::support::Test;

mod support;

/// The variables the trampoline sets for the compiler it runs.
const VARS: [&str; 3] = ["SDKROOT", "CPATH", "LIBRARY_PATH"];

fn host() -> String {
    format!("{}-apple-darwin", std::env::consts::ARCH)
}

fn run(cmd: &mut Command) -> String {
    let output = cmd.output().unwrap();
    assert!(output.status.success(), "{cmd:?} failed");
    String::from_utf8(output.stdout).unwrap()
}

/// `VARS` as a compiler run through the trampoline sees them, from
/// `xcrun env -0` in `env`.
fn trampoline_env(env: &[(OsString, OsString)]) -> Vec<(String, String)> {
    let out = run(Command::new("xcrun")
        .args(["env", "-0"])
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k, v))));
    let mut vars: Vec<(String, String)> = out
        .split('\0')
        .filter_map(|kv| kv.split_once('='))
        .filter(|(k, _)| VARS.contains(k))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    vars.sort();
    vars
}

/// `VARS` as a command cc built sets them.
fn command_env(cmd: &Command) -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = cmd
        .get_envs()
        .filter_map(|(k, v)| Some((k.to_str()?, v?.to_str()?)))
        .filter(|(k, _)| VARS.contains(k))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    vars.sort();
    vars
}

fn compiler(test: &Test, env: &[(OsString, OsString)]) -> cc::Tool {
    let mut build = cc::Build::new();
    build
        .target(&host())
        .host(&host())
        .opt_level(0)
        .debug(false)
        .cargo_metadata(false)
        .out_dir(test.td.path());
    build.set_envs_snapshot(env.iter().map(|(k, v)| (k.as_os_str(), v.as_os_str())));
    build.try_get_compiler().unwrap()
}

/// Whether `PATH` in `env` finds Apple's `/usr/bin/cc` first.
fn path_finds_trampoline(env: &[(OsString, OsString)]) -> bool {
    let Some((_, path)) = env.iter().find(|(k, _)| k == "PATH") else {
        return false;
    };
    std::env::split_paths(path)
        .map(|dir| dir.join("cc"))
        .find(|cc| cc.is_file())
        .map_or(false, |cc| cc == Path::new("/usr/bin/cc"))
}

#[test]
fn runs_the_compiler_the_trampoline_runs() {
    let test = Test::new();
    let base: Vec<(OsString, OsString)> = test
        .env
        .iter()
        .filter(|(k, _)| !VARS.contains(&k.to_str().unwrap_or_default()) && *k != "DEVELOPER_DIR")
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    if !path_finds_trampoline(&base) {
        return;
    }

    // With `SDKROOT` unset the trampoline sets it and adds `/usr/local` to
    // `CPATH` and `LIBRARY_PATH`, after any value of theirs; with `SDKROOT`
    // set it adds nothing. `DEVELOPER_DIR` picks the developer directory.
    let sdk = run(Command::new("xcrun").args(["--show-sdk-path", "--sdk", "macosx"]));
    let mut cases = vec![
        vec![],
        vec![("SDKROOT", sdk.trim())],
        vec![("CPATH", "/opt/include"), ("LIBRARY_PATH", "/opt/lib")],
    ];
    let clt = "/Library/Developer/CommandLineTools";
    if Path::new(clt).join("usr/bin/cc").exists() {
        cases.push(vec![("DEVELOPER_DIR", clt)]);
    }
    for case in cases {
        let mut env = base.clone();
        env.extend(case.iter().map(|(k, v)| (k.into(), v.into())));
        let real = run(Command::new("xcrun")
            .args(["--find", "cc"])
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v))));
        let tool = compiler(&test, &env);
        assert_eq!(tool.path(), Path::new("cc"), "callers still get `cc`");
        let cmd = tool.to_command();
        assert_eq!(
            Path::new(cmd.get_program()),
            Path::new(real.trim()),
            "{case:?}"
        );
        assert_eq!(command_env(&cmd), trampoline_env(&env), "{case:?}");
    }
}

#[test]
fn keeps_a_cc_found_earlier_in_path() {
    let test = Test::new();
    let env: Vec<(OsString, OsString)> = test
        .env
        .iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    if !path_finds_trampoline(&env) {
        return;
    }
    // Like a ccache or Xcode directory ahead of `/usr/bin` in `PATH`.
    let dir = test.td.path().join("bin");
    std::fs::create_dir(&dir).unwrap();
    symlink("/usr/bin/cc", dir.join("cc")).unwrap();
    let env: Vec<(OsString, OsString)> = env
        .into_iter()
        .map(|(k, v)| {
            if k == "PATH" {
                let mut path = dir.clone().into_os_string();
                path.push(":");
                path.push(v);
                (k, path)
            } else {
                (k, v)
            }
        })
        .collect();
    let cmd = compiler(&test, &env).to_command();
    assert_eq!(cmd.get_program(), OsStr::new("cc"));
}
