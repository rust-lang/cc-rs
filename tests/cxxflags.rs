mod support;

use crate::support::Test;

#[test]
fn gnu_no_warnings_if_cxxflags() {
    let mut test = Test::gnu();
    test.env.set("CXXFLAGS", "-arbitrary");
    test.gcc().file("foo.cpp").cpp(true).compile("foo");

    test.cmd(0).must_not_have("-Wall").must_not_have("-Wextra");
}
