#![allow(missing_docs)]

mod support;

use std::ffi::OsStr;

use crate::support::Test;

#[test]
fn flag_propagates_to_compiler() {
    let compiler = Test::new().gcc().flag("--foo").get_compiler();

    assert!(compiler.args().contains(&"--foo".into()));

    assert!(compiler.to_command().get_args().any(|flag| flag == "--foo"));
}

#[test]
fn env_propagates_to_compiler() {
    let compiler = Test::new().gcc().env("FOO", "BAR").get_compiler();

    assert!(compiler
        .get_envs()
        .any(|(key, val)| key == "FOO" && val == "BAR"));

    assert!(compiler
        .to_command()
        .get_envs()
        .any(|(key, val)| key == "FOO" && val.unwrap() == "BAR"));
}

/// The deprecated `Tool::env` returns what `Tool::get_envs` yields, in the
/// same order, also after cc ran a flag probe while building the `Tool`.
#[test]
#[allow(deprecated)]
fn env_matches_get_envs() {
    let compiler = Test::gnu()
        .gcc()
        .env("FOO", "BAR")
        .env("FOO", "BAZ")
        .flag_if_supported("-Wall")
        .get_compiler();

    let last_two: Vec<_> = compiler.get_envs().rev().take(2).collect();
    assert_eq!(
        last_two,
        [
            (OsStr::new("FOO"), OsStr::new("BAZ")),
            (OsStr::new("FOO"), OsStr::new("BAR"))
        ]
    );

    let envs: Vec<_> = compiler
        .get_envs()
        .map(|(key, val)| (key.to_owned(), val.to_owned()))
        .collect();
    assert_eq!(compiler.env(), envs);
}

#[test]
fn env_propagates_to_archiver() {
    let archiver = Test::new().gcc().env("FOO", "BAR").get_archiver();

    assert!(archiver
        .get_envs()
        .any(|(key, val)| key == "FOO" && val.unwrap() == "BAR"));
}

#[test]
fn env_propagates_to_ranlib() {
    let ranlib = Test::new().gcc().env("FOO", "BAR").get_ranlib();

    assert!(ranlib
        .get_envs()
        .any(|(key, val)| key == "FOO" && val.unwrap() == "BAR"));
}
