mod support;

use crate::support::Test;

#[test]
fn gnu_test_parse_shell_escaped_flags() {
    let mut test = Test::gnu();
    test.env.set("CFLAGS", "foo \"bar baz\"");
    test.env.set("CC_SHELL_ESCAPED_FLAGS", "1");
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0).must_have("foo").must_have("bar baz");
}

#[test]
fn gnu_test_parse_shell_escaped_flags2() {
    let mut test = Test::gnu();
    test.env.set("CFLAGS", "foo \"bar baz\"");
    test.env.remove("CC_SHELL_ESCAPED_FLAGS");
    test.gcc().file("foo.c").compile("foo");

    test.cmd(0)
        .must_have("foo")
        .must_have_in_order("\"bar", "baz\"");
}
