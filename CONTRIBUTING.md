# Contributing to cc-rs

Thanks for helping improve `cc`. This crate is a
[Cargo build-script](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
helper: it invokes the platform compiler so C/C++/assembly/CUDA can be linked
into a Rust crate.

## Pull requests

- Open pull requests against `main`.
- Add or update tests when behavior changes. Integration tests live in
  `tests/`; the workspace also includes `find-msvc-tools` and tools under
  `dev-tools/`.
- Before you push, run at least:

  ```bash
  cargo test
  cargo fmt -- --check
  ```

  On macOS, `cargo test` may need the Xcode Command Line Tools and `SDKROOT`;
  see [Testing](README.md#testing).

  CI additionally runs Clippy, the MSRV toolchain (see `rust-version` in
  `Cargo.toml`), and `tombi format --check` for TOML.
- Keep diffs focused. Do not reformat unrelated code.

## Conventional Commits

We **attempt** to follow
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
so [release-plz](https://release-plz.dev/) / git-cliff can generate
`CHANGELOG.md` from commit subjects.

PRs are typically squash-merged, so the **PR title** becomes the changelog
subject. CI checks that the title looks like a Conventional Commits subject:

```text
type: description
type(scope): description
type!: breaking description
```

Types are lowercase. A `!` before the colon marks a breaking change.

This check looks at the PR title only (not the full git history). Linting
every historical commit on a mature repo would be too noisy.

### Prefixes that affect the changelog

These match the `commit_parsers` in [`release-plz.toml`](release-plz.toml):

| Prefix | Changelog section |
| --- | --- |
| `feat:` / `feat(...):` | Added |
| `fix:` / `fix(...):` | Fixed |
| `security:` / `security(...):` | Security |
| `changed` / `changed:` / `changed(...):` | Changed |
| `deprecated` / `deprecated:` / `deprecated(...):` | Deprecated |
| `removed` / `removed:` / `removed(...):` | Removed |
| `ci:` / `ci(...):` | *skipped* (not user-facing) |
| `chore:` / `chore(...):` | *skipped* |
| `refactor:` / `refactor(...):` | *skipped* |
| anything else | Other |

`changed` / `deprecated` / `removed` match subjects that **start with** those
words, as the parsers are written. Prefer the conventional form with a colon
(`changed: …`) so the title stays valid for CI.

Use `ci:`, `chore:`, or `refactor:` (including scoped forms) for work that
should not appear in the user-facing changelog: CI, tooling, and internal
cleanups. Dependabot GitHub Actions bumps use `ci(deps):` on purpose so they
are skipped the same way.

Other Conventional Commits types (`docs:`, `test:`, `perf:`, `build:`, …) are
fine; they land under **Other** unless they match a row above.
