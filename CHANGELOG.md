# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.101](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.100...cc-v1.0.101) - 2024-06-25

### Other
- Use `Build::getenv` instead of `env::var*` in anywhere that makes sense ([#1103](https://github.com/rust-lang/cc-rs/pull/1103))

## [1.0.100](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.99...cc-v1.0.100) - 2024-06-23

### Other
- Update publish.yml to use release-plz ([#1101](https://github.com/rust-lang/cc-rs/pull/1101))
- Accpet `OsStr` instead of `str` for flags ([#1100](https://github.com/rust-lang/cc-rs/pull/1100))
- Use `dep:` syntax to avoid implicit features. ([#1099](https://github.com/rust-lang/cc-rs/pull/1099))
- Minor clippy fixes. ([#1098](https://github.com/rust-lang/cc-rs/pull/1098))
- Fix WASI compilation for C++ ([#1083](https://github.com/rust-lang/cc-rs/pull/1083))
- Regenerate windows sys bindings ([#1096](https://github.com/rust-lang/cc-rs/pull/1096))
- Rename regenerate-windows-sys to regenerate-windows-sys.yml ([#1095](https://github.com/rust-lang/cc-rs/pull/1095))
- Create regenerate-windows-sys.yml ([#1094](https://github.com/rust-lang/cc-rs/pull/1094))
- Update windows-bindgen requirement from 0.56 to 0.57 ([#1091](https://github.com/rust-lang/cc-rs/pull/1091))
- Eagerly close tempfile to fix [#1082](https://github.com/rust-lang/cc-rs/pull/1082) ([#1087](https://github.com/rust-lang/cc-rs/pull/1087))
- Output msvc.exe in the output directory ([#1090](https://github.com/rust-lang/cc-rs/pull/1090))
- Fix clippy warnings on Windows ([#1088](https://github.com/rust-lang/cc-rs/pull/1088))
- Don't try to free DLL on drop ([#1089](https://github.com/rust-lang/cc-rs/pull/1089))
- Fix panic safety issue in StderrForwarder ([#1079](https://github.com/rust-lang/cc-rs/pull/1079))
