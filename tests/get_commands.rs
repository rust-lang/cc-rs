#![allow(missing_docs)]

mod support;

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

    assert!(compiler.env().contains(&("FOO".into(), "BAR".into())));

    assert!(compiler
        .to_command()
        .get_envs()
        .any(|(key, val)| key == "FOO" && val.unwrap() == "BAR"));
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
