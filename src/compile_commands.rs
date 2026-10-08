//! Record the commands cc compiles with as a
//! [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
//! the `compile_commands.json` that clangd, clang-tidy and other tools read.
//!
//! cc passes each compile command to the logger set with
//! [`Build::message_logger`] as a [`BuildMessageKind::CompileCommand`]
//! message. A [`CompileCommandCollector`] keeps them, and
//! [`store_json_compilation_database`] writes them to a file:
//!
//! ```no_run
//! use std::{env, path::PathBuf, sync::Arc};
//! use cc::compile_commands::{store_json_compilation_database, CompileCommandCollector};
//!
//! let collector = Arc::new(CompileCommandCollector::new());
//! cc::Build::new()
//!     .file("src/foo.c")
//!     .message_logger(Some(collector.clone()))
//!     .compile("foo");
//! cc::Build::new()
//!     .cpp(true)
//!     .file("src/bar.cpp")
//!     .message_logger(Some(collector.clone()))
//!     .compile("bar");
//!
//! let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
//! store_json_compilation_database(&collector.commands(), out_dir.join("compile_commands.json"))
//!     .unwrap();
//! ```
//!
//! Cargo expects build scripts to write only to `OUT_DIR`, which is inside the
//! target folder, so point the tool there, for example with clangd's
//! `--compile-commands-dir`.
//!
//! # What is recorded
//!
//! Every object file cc compiles gets one entry, in the order cc starts the
//! compiles, also with the `parallel` feature. That covers C, C++, CUDA and
//! assembly sources, including `.asm` files that MSVC targets assemble with
//! MASM, armasm, `llvm-ml` or `CC_MASM_ASM`. An entry is logged right before cc
//! starts its command, so a failed build still has the entries of the compiles
//! it started. Each entry holds:
//!
//! - `arguments`: the program and its arguments exactly as cc runs them. That
//!   includes a compiler wrapper such as `sccache` from `RUSTC_WRAPPER` or
//!   `CC`; clang's tools skip `ccache`, `sccache` and `distcc` themselves.
//! - `directory`: the folder the compiler runs in, which is the build
//!   script's current directory (the package's folder when Cargo runs it).
//! - `file` and `output`: the source file and the object file, made absolute
//!   by joining them to `directory`.
//!
//! Not recorded are the environment variables cc sets for the compiler, such
//! as `INCLUDE` and `PATH` for MSVC, since the format has no place for them,
//! and commands that don't compile a source file into an object: the
//! archiver, CUDA device linking, [`Build::expand`], flag support checks and
//! compiler detection.
//!
//! JSON text is Unicode, so [`store_json_compilation_database`] writes a path
//! or argument that isn't valid Unicode with replacement characters (U+FFFD),
//! as [`OsStr::to_string_lossy`] does. [`CompileCommand`] keeps the exact
//! values.
//!
//! [`Build::message_logger`]: crate::Build::message_logger
//! [`Build::expand`]: crate::Build::expand

use std::{
    any::Any,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{logger::Logger, BuildMessage, BuildMessageKind, BuildMessageLogger};

/// The command that compiles one object file: an entry of a
/// [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html).
///
/// cc passes it as the `extra` of a [`BuildMessageKind::CompileCommand`]
/// message. See the [module docs](self) for what each field holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileCommand {
    directory: PathBuf,
    file: PathBuf,
    arguments: Vec<OsString>,
    output: PathBuf,
}

impl CompileCommand {
    /// `cmd` compiles `src` into `dst` and runs in `directory`, which must be
    /// absolute.
    pub(crate) fn new(cmd: &Command, src: &Path, dst: &Path, directory: &Path) -> Self {
        // cc runs its compile commands in its own working directory.
        debug_assert!(cmd.get_current_dir().is_none());
        Self {
            directory: directory.to_path_buf(),
            file: directory.join(src),
            arguments: std::iter::once(cmd.get_program())
                .chain(cmd.get_args())
                .map(OsStr::to_os_string)
                .collect(),
            output: directory.join(dst),
        }
    }

    /// The working directory of the compiler, an absolute path. Relative paths
    /// in the arguments are relative to it.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The source file, an absolute path.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The program cc runs, followed by its arguments, without any escaping.
    pub fn arguments(&self) -> impl ExactSizeIterator<Item = &OsStr> + DoubleEndedIterator {
        self.arguments.iter().map(|arg| &**arg)
    }

    /// The object file, an absolute path.
    pub fn output(&self) -> &Path {
        &self.output
    }
}

/// A [`BuildMessageLogger`] that keeps the [`CompileCommand`] of every
/// [`BuildMessageKind::CompileCommand`] message, to write them with
/// [`store_json_compilation_database`].
///
/// Set it on each `Build` with
/// [`Build::message_logger`](crate::Build::message_logger). Several `Build`s
/// and their clones can share one collector, also across threads; it keeps the
/// commands in the order cc logged them. A `Build` that compiles twice is
/// recorded twice.
///
/// The collector doesn't change what cc prints: its warnings and the
/// compiler's stderr are still printed as `cargo:warning=` lines unless
/// [`Build::cargo_warnings`](crate::Build::cargo_warnings) is off. To also
/// handle the messages in a logger of your own, pass it to
/// [`forward_to`](Self::forward_to).
#[derive(Debug, Default)]
pub struct CompileCommandCollector {
    commands: Mutex<Vec<CompileCommand>>,
    forward_to: Option<Logger>,
}

impl CompileCommandCollector {
    /// A collector with no commands yet that passes messages on to no other
    /// logger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pass every message, compile commands included, on to `logger` as well.
    pub fn forward_to(mut self, logger: Arc<dyn BuildMessageLogger>) -> Self {
        self.forward_to = Some(Logger(logger));
        self
    }

    /// A copy of the commands collected so far, in the order cc logged them.
    pub fn commands(&self) -> Vec<CompileCommand> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<CompileCommand>> {
        // Nothing under the lock can leave the list half changed.
        self.commands.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl BuildMessageLogger for CompileCommandCollector {
    fn log(&self, kind: BuildMessageKind, msg: BuildMessage<'_>, extra: &dyn Any) {
        if kind == BuildMessageKind::CompileCommand {
            if let Some(command) = extra.downcast_ref::<CompileCommand>() {
                self.lock().push(command.clone());
            }
        }
        if let Some(logger) = &self.forward_to {
            logger.0.log(kind, msg, extra);
        }
    }
}

/// Write `commands` to `path` as a
/// [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
/// replacing the file if it exists.
///
/// Paths and arguments that aren't valid Unicode are written with
/// replacement characters (U+FFFD). See the [module docs](self) for an
/// example.
///
/// # Errors
///
/// Returns the error of writing the file, for example when its folder doesn't
/// exist.
pub fn store_json_compilation_database<'a, C, P>(commands: C, path: P) -> io::Result<()>
where
    C: IntoIterator<Item = &'a CompileCommand>,
    P: AsRef<Path>,
{
    fs::write(
        path.as_ref(),
        json_compilation_database(&mut commands.into_iter()),
    )
}

/// The JSON text of a compilation database holding `commands`.
fn json_compilation_database(commands: &mut dyn Iterator<Item = &CompileCommand>) -> String {
    let mut json = String::from("[");
    for (i, command) in commands.enumerate() {
        json.push_str(if i == 0 { "\n" } else { ",\n" });
        json.push_str("  {\n    \"directory\": ");
        push_json_string(&mut json, command.directory.as_os_str());
        json.push_str(",\n    \"file\": ");
        push_json_string(&mut json, command.file.as_os_str());
        json.push_str(",\n    \"arguments\": [");
        for (i, argument) in command.arguments.iter().enumerate() {
            if i > 0 {
                json.push_str(", ");
            }
            push_json_string(&mut json, argument);
        }
        json.push_str("],\n    \"output\": ");
        push_json_string(&mut json, command.output.as_os_str());
        json.push_str("\n  }");
    }
    json.push_str("\n]\n");
    json
}

/// Append `s` to `json` as a JSON string.
fn push_json_string(json: &mut String, s: &OsStr) {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    json.push('"');
    for c in s.to_string_lossy().chars() {
        match c {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            // JSON strings can't hold control characters as they are.
            '\0'..='\x1f' => {
                json.push_str("\\u00");
                json.push(char::from(HEX_DIGITS[c as usize >> 4]));
                json.push(char::from(HEX_DIGITS[c as usize & 0xf]));
            }
            c => json.push(c),
        }
    }
    json.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(directory: &str, file: &str, arguments: &[&str], output: &str) -> CompileCommand {
        CompileCommand {
            directory: directory.into(),
            file: file.into(),
            arguments: arguments.iter().map(OsString::from).collect(),
            output: output.into(),
        }
    }

    #[test]
    fn paths_are_joined_to_the_directory() {
        let directory = Path::new(if cfg!(windows) { r"C:\crate" } else { "/crate" });
        let mut cmd = Command::new("cc");
        cmd.args(["-c", "src/foo.c", "-o", "out/foo.o"]);

        let command = CompileCommand::new(
            &cmd,
            Path::new("src/foo.c"),
            Path::new("out/foo.o"),
            directory,
        );
        assert_eq!(command.directory(), directory);
        assert_eq!(command.file(), directory.join("src/foo.c"));
        assert_eq!(command.output(), directory.join("out/foo.o"));
        assert!(command
            .arguments()
            .eq(["cc", "-c", "src/foo.c", "-o", "out/foo.o"]));

        // An absolute source stays as it is.
        let src = directory.join("other").join("bar.c");
        let command = CompileCommand::new(&cmd, &src, Path::new("out/bar.o"), directory);
        assert_eq!(command.file(), src);
    }

    #[test]
    fn json_of_no_commands_is_an_empty_array() {
        assert_eq!(json_compilation_database(&mut [].iter()), "[\n]\n");
    }

    #[test]
    fn json_escapes_quotes_backslashes_and_control_characters() {
        let commands = [
            command(
                r"C:\Users\me\crate",
                r"C:\Users\me\crate\src\foo.c",
                &[
                    r"C:\Program Files\LLVM\bin\clang-cl.exe",
                    r#"-DGREETING="hello world""#,
                    "-DTAB=\t",
                    "-DCONTROL=\0\x1b\x1f",
                    "-DKEPT=ü",
                ],
                r"C:\Users\me\crate\target\out\foo.o",
            ),
            command(
                "/crate",
                "/crate/bar.c",
                &["cc", "-c", "bar.c"],
                "/out/bar.o",
            ),
        ];
        let expected = r#"[
  {
    "directory": "C:\\Users\\me\\crate",
    "file": "C:\\Users\\me\\crate\\src\\foo.c",
    "arguments": ["C:\\Program Files\\LLVM\\bin\\clang-cl.exe", "-DGREETING=\"hello world\"", "-DTAB=\u0009", "-DCONTROL=\u0000\u001b\u001f", "-DKEPT=ü"],
    "output": "C:\\Users\\me\\crate\\target\\out\\foo.o"
  },
  {
    "directory": "/crate",
    "file": "/crate/bar.c",
    "arguments": ["cc", "-c", "bar.c"],
    "output": "/out/bar.o"
  }
]
"#;
        assert_eq!(json_compilation_database(&mut commands.iter()), expected);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn json_replaces_invalid_unicode() {
        #[cfg(unix)]
        let invalid = {
            use std::os::unix::ffi::OsStrExt;
            OsStr::from_bytes(b"foo\xff.c").to_os_string()
        };
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            // An unpaired surrogate.
            OsString::from_wide(&[0x66, 0x6f, 0x6f, 0xd800, 0x2e, 0x63])
        };
        let mut json = String::new();
        push_json_string(&mut json, &invalid);
        assert_eq!(json, "\"foo\u{fffd}.c\"");
    }
}
