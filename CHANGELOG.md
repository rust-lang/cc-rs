# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.10](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.9...cc-v1.1.10) - 2024-08-11

### Other
- Remap Windows targets triples to their LLVM counterparts ([#1176](https://github.com/rust-lang/cc-rs/pull/1176))

## [1.1.9](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.8...cc-v1.1.9) - 2024-08-11

### Other
- Add custom CC wrapper to the wrapper whitelist ([#1175](https://github.com/rust-lang/cc-rs/pull/1175))

## [1.1.8](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.7...cc-v1.1.8) - 2024-08-06

### Other
- Fix broken link in docs.rs ([#1173](https://github.com/rust-lang/cc-rs/pull/1173))

## [1.1.7](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.6...cc-v1.1.7) - 2024-07-29

### Other
- add `.objects` ([#1166](https://github.com/rust-lang/cc-rs/pull/1166))

## [1.1.6](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.5...cc-v1.1.6) - 2024-07-19

### Other
- Clippy fixes ([#1163](https://github.com/rust-lang/cc-rs/pull/1163))

## [1.1.5](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.4...cc-v1.1.5) - 2024-07-15

### Other
- Fix cyclic compilation: Use vendored once_cell ([#1154](https://github.com/rust-lang/cc-rs/pull/1154))

## [1.1.4](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.3...cc-v1.1.4) - 2024-07-14

### Other
- Support compiling on wasm targets (Supersede [#1068](https://github.com/rust-lang/cc-rs/pull/1068)) ([#1160](https://github.com/rust-lang/cc-rs/pull/1160))

## [1.1.3](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.2...cc-v1.1.3) - 2024-07-14

### Other
- Reduce msrv to 1.63 ([#1158](https://github.com/rust-lang/cc-rs/pull/1158))
- Revert "Use raw-dylib for windows-sys ([#1137](https://github.com/rust-lang/cc-rs/pull/1137))" ([#1157](https://github.com/rust-lang/cc-rs/pull/1157))
- Fix typos ([#1152](https://github.com/rust-lang/cc-rs/pull/1152))
- Fix `doc_lazy_continuation` lints ([#1153](https://github.com/rust-lang/cc-rs/pull/1153))

## [1.1.2](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.1...cc-v1.1.2) - 2024-07-12

### Other
- Add empty `jobserver` feature. ([#1150](https://github.com/rust-lang/cc-rs/pull/1150))

## [1.1.1](https://github.com/rust-lang/cc-rs/compare/cc-v1.1.0...cc-v1.1.1) - 2024-07-12

### Other
- Fix is_flag_supported not respecting emit_rerun_if_env_changed ([#1147](https://github.com/rust-lang/cc-rs/pull/1147)) ([#1148](https://github.com/rust-lang/cc-rs/pull/1148))

## [1.1.0](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.106...cc-v1.1.0) - 2024-07-08

### Added
- add cargo_output to eliminate last vestiges of stdout pollution ([#1141](https://github.com/rust-lang/cc-rs/pull/1141))

## [1.0.106](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.105...cc-v1.0.106) - 2024-07-08

### Other
- Drop support for Visual Studio 12 (2013) ([#1046](https://github.com/rust-lang/cc-rs/pull/1046))
- Use raw-dylib for windows-sys ([#1137](https://github.com/rust-lang/cc-rs/pull/1137))
- Bump msrv to 1.67 ([#1143](https://github.com/rust-lang/cc-rs/pull/1143))
- Bump msrv to 1.65 ([#1140](https://github.com/rust-lang/cc-rs/pull/1140))
- Fix clippy warnings ([#1138](https://github.com/rust-lang/cc-rs/pull/1138))

## [1.0.105](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.104...cc-v1.0.105) - 2024-07-07

### Other
- Regenerate windows sys bindings ([#1132](https://github.com/rust-lang/cc-rs/pull/1132))
- Fix generate-windows-sys-bindings ([#1133](https://github.com/rust-lang/cc-rs/pull/1133))
- Fix gen-windows-sys-binding ([#1130](https://github.com/rust-lang/cc-rs/pull/1130))
- Fix gen-windows-sys-binding ([#1127](https://github.com/rust-lang/cc-rs/pull/1127))
- Update windows-bindgen requirement from 0.57 to 0.58 ([#1123](https://github.com/rust-lang/cc-rs/pull/1123))

## [1.0.104](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.103...cc-v1.0.104) - 2024-07-01

### Other
- Fixed link break about compile-time-requirements ([#1118](https://github.com/rust-lang/cc-rs/pull/1118))

## [1.0.103](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.102...cc-v1.0.103) - 2024-06-30

### Other
- Fix compilation for wasm: env WASI_SYSROOT should be optional ([#1114](https://github.com/rust-lang/cc-rs/pull/1114))

## [1.0.102](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.101...cc-v1.0.102) - 2024-06-29

### Other
- Fix invalid wasi targets compatibility ([#1105](https://github.com/rust-lang/cc-rs/pull/1105))
- Speedup regenerate-target-info and regenerate-windows-sys ([#1110](https://github.com/rust-lang/cc-rs/pull/1110))

## [1.0.101](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.100...cc-v1.0.101) - 2024-06-25

### Other
- Use `Build::getenv` instead of `env::var*` in anywhere that makes sense ([#1103](https://github.com/rust-lang/cc-rs/pull/1103))

## [1.0.100](https://github.com/rust-lang/cc-rs/compare/cc-v1.0.99...cc-v1.0.100) - 2024-06-23

### Other
- Update publish.yml to use release-plz ([#1101](https://github.com/rust-lang/cc-rs/pull/1101))
- Accept `OsStr` instead of `str` for flags ([#1100](https://github.com/rust-lang/cc-rs/pull/1100))
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
