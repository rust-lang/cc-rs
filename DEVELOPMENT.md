# Development

Notes for working on `cc-rs` locally. For pull request and commit conventions,
see [CONTRIBUTING.md](CONTRIBUTING.md).

## Setup

On macOS, install the
[Xcode Command Line Tools](https://developer.apple.com/documentation/xcode/installing-the-command-line-tools#Install-the-Command-Line-Tools-package-in-Terminal)
before developing or running tests:

```sh
xcode-select --install
```

## Testing

The default check is:

```sh
cargo test
```

Before you push, also run `cargo fmt -- --check` (see
[CONTRIBUTING.md](CONTRIBUTING.md)). CI runs Clippy, the MSRV toolchain, and
other checks described there.

On macOS, Apple targets locate an SDK through `xcrun` and `SDKROOT`; that
logic lives in [`src/lib.rs`](src/lib.rs). If `cargo test` fails with
`xcrun: SDK "appletvos" cannot be located` (or a similar SDK lookup error),
point `SDKROOT` at the macOS SDK and rerun tests:

```sh
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
cargo test
```
