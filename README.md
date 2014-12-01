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

Currently use of this crate means that Windows users will require gcc to be
installed at compile-time. This is typically acquired through the
[MinGW-w64](http://mingw-w64.sourceforge.net), although we recommend installing
through [MSYS2 instead][msys2]. Make sure to install the appropriate
architecture corresponding to your installation of rustc.

[msys2]: https://github.com/rust-lang/rust/wiki/Note-getting-started-developing-Rust#windows

Once gcc is installed, it also requires that the directory containing gcc is in
the PATH environment variable.

# License

`gcc-rs` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
