mod support;
use std::ffi::OsStr;

use crate::support::Test;

#[test]
fn flag_propagates_to_compiler() {
    let compiler = Test::new().gcc().flag("--foo").get_compiler();

    assert!(compiler.args().contains(&"--foo".into()));

    let predicate = |arg: &&OsStr| *arg == "--foo";
    assert!(compiler.to_command().get_args().find(predicate).is_some());
}

#[test]
fn env_propagates_to_compiler() {
    let compiler = Test::new().gcc().env("FOO", "BAR").get_compiler();

    assert!(compiler.env().contains(&("FOO".into(), "BAR".into())));

    let predicate = |(key, val): &(&OsStr, Option<&OsStr>)| *key == "FOO" && val.unwrap() == "BAR";
    assert!(compiler.to_command().get_envs().find(predicate).is_some());
}

#[test]
fn env_propagates_to_archiver() {
    let archiver = Test::new().gcc().env("FOO", "BAR").get_archiver();

    let predicate = |(key, val): &(&OsStr, Option<&OsStr>)| *key == "FOO" && val.unwrap() == "BAR";
    assert!(archiver.get_envs().find(predicate).is_some());
}

#[test]
fn env_propagates_to_ranlib() {
    let ranlib = Test::new().gcc().env("FOO", "BAR").get_ranlib();

    let predicate = |(key, val): &(&OsStr, Option<&OsStr>)| *key == "FOO" && val.unwrap() == "BAR";
    assert!(ranlib.get_envs().find(predicate).is_some());
}
