# gcc-rs

[![Build Status](https://travis-ci.org/alexcrichton/gcc-rs.svg?branch=master)](https://travis-ci.org/alexcrichton/gcc-rs)
[![Build status](https://ci.appveyor.com/api/projects/status/onu270iw98h81nwv?svg=true)](https://ci.appveyor.com/project/alexcrichton/gcc-rs)

[Documentation](http://alexcrichton.com/gcc-rs/gcc/index.html)

A simple library meant to be used as a build dependency with Cargo packages in
order to build a set of C files into a static archive.

```rust,no_run
extern crate gcc;

fn main() {
    gcc::compile_library("libfoo.a", &["foo.c", "bar.c"]);
}
```

# External configuration via environment variables

To control the programs and flags used for building, the builder can set a
number of different environment variables.

* `CFLAGS` - a series of space seperated flags passed to "gcc". Note that
             individual flags cannot currently contain spaces, so doing
             something like: "-L=foo\ bar" is not possible.
* `CC` - the actual C compiler used. Note that this is used as an exact
         executable name, so (for example) no extra flags can be passed inside
         this variable, and the builder must ensure that there aren't any
         trailing spaces. This compiler must understand the `-c` flag. For
         certain `TARGET`s, it also is assumed to know about other flags (most
         common is `-fPIC`).
* `AR` - the `ar` (archiver) executable to use to build the static library.

Each of these variables can also be supplied with certain prefixes and suffixes,
in the following prioritized order:

1. `<var>_<target>` - for example, `CC_x86_64-unknown-linux-gnu`
2. `<var>_<target_with_underscores>` - for example, `CC_x86_64_unknown_linux_gnu`
3. `<build-kind>_<var>` - for example, `HOST_CC` or `TARGET_CFLAGS`
4. `<var>` - a plain `CC`, `AR` as above.

If none of these varaibles exist, gcc-rs uses built-in defaults

In addition to the the above optional environment variables, `gcc-rs` has some
functions with hard requirements on some variables supplied by [cargo's
build-script driver][cargo] that it has the `TARGET`, `OUT_DIR`, `OPT_LEVEL`,
and `HOST` variables.

[cargo]: http://doc.crates.io/build-script.html#inputs-to-the-build-script

# Compile-time Requirements

To work properly this crate needs access to a C compiler when the build script
is being run. This crate does not ship a C compiler with it. The compiler
required varies per platform, but there are three broad categories:

* Unix platforms require `cc` to be the C compiler. This can be found by
  installing gcc/clang on Linux distributions and Xcode on OSX, for example.
* Windows platforms targeting MSVC (e.g. your target triple ends in `-msvc`)
  require `cl.exe` to be available and in `PATH`. This is typically found in
  standard Visual Studio installations and the `PATH` can be set up by running
  the appropriate developer tools shell.
* Windows platforms targeting MinGW (e.g. your target triple ends in `-gnu`)
  require `gcc` to be available in `PATH`. We recommend the
  [MinGW-w64](http://mingw-w64.sourceforge.net) distribution
  ([direct link to the installer][mingw-installer]). You may also acquite it via
  [MSYS2](http://msys2.github.io), as explained [here][msys2-help].  Make sure
  to install the appropriate architecture corresponding to your installation of
  rustc.

[mingw-installer]: http://sourceforge.net/projects/mingw-w64/files/latest/download
[msys2-help]: http://github.com/rust-lang/rust#building-on-windows

# C++ support

`gcc-rs` supports C++ libraries compilation by using the `cpp` method on
`Config`:

```rust,no_run
extern crate gcc;

fn main() {
    gcc::Config::new()
        .cpp(true) // Switch to C++ library compilation.
        .file("foo.cpp")
        .compile("libfoo.a");
}
```

When using C++ library compilation switch, the `CXX` and `CXXFLAGS` env
variables are used instead of `CC` and `CFLAGS` and the C++ standard library is
linked to the crate target.

# License

`gcc-rs` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
