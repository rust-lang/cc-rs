//! Record the commands cc compiles with as a
//! [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
//! the `compile_commands.json` that clangd, clang-tidy and other tools read.
//!
//! cc passes each compile command to the logger set with
//! [`Build::message_logger`] as a [`BuildMessageKind::CompileCommand`]
//! message. A [`CompileCommandCollector`] keeps them,
//! [`store_json_compilation_database`] writes them to a file, and
//! [`json_compilation_database`] formats them for any other writer:
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
//! JSON text is Unicode, so a path or argument that isn't valid Unicode is
//! written with replacement characters (U+FFFD), as
//! [`OsStr::to_string_lossy`] does. [`CompileCommand`] keeps the exact values.
//!
//! [`Build::message_logger`]: crate::Build::message_logger
//! [`Build::expand`]: crate::Build::expand

use std::{
    any::Any,
    ffi::{OsStr, OsString},
    fmt, fs,
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{
    logger::Logger, utilities::from_fn, BuildMessage, BuildMessageKind, BuildMessageLogger,
};

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

    /// The commands collected so far, in the order cc logged them.
    ///
    /// The collector stays locked while the returned value is alive, so
    /// logging into it from another thread waits until it is dropped. On the
    /// same thread, compiling with a `Build` that uses this collector or
    /// calling `commands` again would lock it a second time, which may
    /// deadlock or panic like [`Mutex::lock`]. So drop it before either, and
    /// use `to_vec()` to keep a copy.
    pub fn commands(&self) -> impl Deref<Target = [CompileCommand]> + '_ {
        Commands(self.lock())
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

/// The collector's commands, locked while borrowed.
struct Commands<'a>(MutexGuard<'a, Vec<CompileCommand>>);

impl Deref for Commands<'_> {
    type Target = [CompileCommand];

    fn deref(&self) -> &[CompileCommand] {
        &self.0
    }
}

/// Format `commands` as a
/// [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
/// writing straight to the formatter without building a `String` first.
///
/// It can be formatted any number of times and into anything that takes a
/// [`Display`](fmt::Display): `to_string()`, `write!` to a file or a buffer,
/// or as part of a larger text. Paths and arguments that aren't valid Unicode
/// are written with replacement characters (U+FFFD).
/// [`store_json_compilation_database`] writes it to a file.
#[must_use]
pub fn json_compilation_database(commands: &[CompileCommand]) -> impl fmt::Display + '_ {
    from_fn(move |f| write_json_compilation_database(f, commands))
}

/// Write `commands` to `path` as a
/// [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
/// replacing the file if it exists. The text is the one
/// [`json_compilation_database`] formats.
///
/// See the [module docs](self) for an example.
///
/// # Errors
///
/// Returns the error of writing the file, for example when its folder doesn't
/// exist.
pub fn store_json_compilation_database<P: AsRef<Path>>(
    commands: &[CompileCommand],
    path: P,
) -> io::Result<()> {
    fn store(commands: &[CompileCommand], path: &Path) -> io::Result<()> {
        let mut file = io::BufWriter::new(fs::File::create(path)?);
        write!(file, "{}", json_compilation_database(commands))?;
        // Dropping a `BufWriter` ignores errors, so flush it here.
        file.flush()
    }

    store(commands, path.as_ref())
}

fn write_json_compilation_database(
    f: &mut fmt::Formatter<'_>,
    commands: &[CompileCommand],
) -> fmt::Result {
    f.write_str("[")?;
    for (i, command) in commands.iter().enumerate() {
        f.write_str(if i == 0 { "\n" } else { ",\n" })?;
        write_json_entry(f, command)?;
    }
    f.write_str("\n]\n")
}

/// Write `command` as one object of the database's array.
fn write_json_entry(f: &mut fmt::Formatter<'_>, command: &CompileCommand) -> fmt::Result {
    f.write_str("  {\n    \"directory\": ")?;
    write_json_string(f, command.directory.as_os_str())?;
    f.write_str(",\n    \"file\": ")?;
    write_json_string(f, command.file.as_os_str())?;
    f.write_str(",\n    \"arguments\": ")?;
    write_json_string_array(f, &command.arguments)?;
    f.write_str(",\n    \"output\": ")?;
    write_json_string(f, command.output.as_os_str())?;
    f.write_str("\n  }")
}

/// Write `items` as a JSON array of strings, on one line.
fn write_json_string_array(f: &mut fmt::Formatter<'_>, items: &[OsString]) -> fmt::Result {
    f.write_str("[")?;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write_json_string(f, item)?;
    }
    f.write_str("]")
}

/// Write `s` as a JSON string. It is converted with `to_string_lossy`, which
/// only allocates for text that isn't valid Unicode.
fn write_json_string(f: &mut fmt::Formatter<'_>, s: &OsStr) -> fmt::Result {
    let s = s.to_string_lossy();
    f.write_str("\"")?;
    // Text between the characters that need escaping is written in one piece.
    let mut unescaped = 0;
    for (i, c) in s.char_indices() {
        if c == '"' || c == '\\' || c < ' ' {
            f.write_str(&s[unescaped..i])?;
            if c < ' ' {
                // JSON strings can't hold control characters as they are.
                write!(f, "\\u{:04x}", u32::from(c))?;
            } else {
                write!(f, "\\{c}")?;
            }
            unescaped = i + c.len_utf8();
        }
    }
    f.write_str(&s[unescaped..])?;
    f.write_str("\"")
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
        assert_eq!(json_compilation_database(&[]).to_string(), "[\n]\n");
    }

    #[test]
    fn json_escapes_quotes_backslashes_and_control_characters() {
        let commands = [
            command(
                r"C:\Users\me\crate",
                r"C:\Users\me\crate\src\foo.c",
                &[
                    r"C:\Program Files\LLVM\bin\clang-cl.exe",
                    r"\\server\share\foo.c",
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
    "arguments": ["C:\\Program Files\\LLVM\\bin\\clang-cl.exe", "\\\\server\\share\\foo.c", "-DGREETING=\"hello world\"", "-DTAB=\u0009", "-DCONTROL=\u0000\u001b\u001f", "-DKEPT=ü"],
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
        assert_eq!(json_compilation_database(&commands).to_string(), expected);
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
        let json = from_fn(|f| write_json_string(f, &invalid)).to_string();
        assert_eq!(json, "\"foo\u{fffd}.c\"");
    }
}
