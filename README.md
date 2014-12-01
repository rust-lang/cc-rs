# gcc-rs

A simple library meant to be used as a build dependency with Cargo packages in
order to build a set of C files into a static archive.

```rust
extern crate gcc;

fn main() {
    gcc::compile_library("libfoo.a", &["foo.c", "bar.c"]);
}
```

# Windows notes

You will need an installation of gcc for this to work. The MinGW-w64 project provides a compiler toolchain for Windows. Make sure to install the appropriate architecture corresponding to your installation of rustc.

http://mingw-w64.sourceforge.net/

Make sure that the installation directory containing gcc is in your PATH environment variable.

# License

`gcc-rs` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
