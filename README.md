# cc-rs

A library for
[Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
to compile a set of C/C++/assembly/CUDA files into a static archive for Cargo to
link into the crate being built. This crate does not compile code itself; it
calls out to the default compiler for the platform. This crate will
automatically detect situations such as cross compilation and various
environment variables and will build code appropriately.

Refer to the [documentation](https://docs.rs/cc) for detailed usage
instructions.

## Testing

On macOS, install the
[Xcode Command Line Tools](https://developer.apple.com/documentation/xcode/installing-the-command-line-tools#Install-the-Command-Line-Tools-package-in-Terminal)
before developing or running tests:

```sh
xcode-select --install
```

Apple targets locate an SDK through `xcrun` and `SDKROOT` (see the Apple
SDK handling in `src/lib.rs`). If `cargo test` fails with `xcrun: SDK
"appletvos" cannot be located` (or a similar SDK lookup error), export the
macOS SDK path and run the tests again:

```sh
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
cargo test
```

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/license/mit)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in cc-rs by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
