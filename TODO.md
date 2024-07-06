- Optimize `Build::try_get_compiler` where we call `Build::is_flag_supported` for each
  `self.flags_supported`.
  Instead, create a new function `Build::filter_supported_flags` to avoid the repeated
  `cc::Build` instance creation.
- Also add parallel support to `Build::filter_supported_flags`/`Build::try_get_compiler`.
  optimize for single flag input.
- Add new fn `Build::flag_if_supported_with_fallbacks`: https://github.com/rust-lang/cc-rs/pull/774
- optimize `Build::compile_objects` for only compiling single object and also add unit test for multi
  objects compilation.

- Add regression test for parallel `compile_objects` with very large number of
  source files

- Fix race condition in `Build::ensure_check_file`
- Update `Build::fix_env_for_apple_os` to use auto generated stuff
