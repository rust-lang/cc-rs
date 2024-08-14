mod support;

use crate::support::Test;
use std::env;

/// This test is in its own module because it modifies the environment and would affect other tests
/// when run in parallel with them.
#[test]
fn gnu_test_parse_shell_escaped_flags() {
    env::set_var("CFLAGS", "foo \"bar baz\"");
    env::set_var("CC_SHELL_ESCAPED_FLAGS", "1");
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_have("foo").must_have("bar baz");

    env::remove_var("CC_SHELL_ESCAPED_FLAGS");
    let test = Test::gnu();
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("foo")
        .must_have_in_order("\"bar", "baz\"");
}
