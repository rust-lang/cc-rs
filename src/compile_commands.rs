//! Record the commands cc compiles with as a
//! [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
//! the `compile_commands.json` that clangd, clang-tidy and other tools read.
//!
//! cc passes each compile command to the logger set with
//! [`Build::message_logger`] as a [`BuildMessageKind::CompileCommand`]
//! message. A [`CompileCommandCollector`] passes them on to its
//! [`CompileCommands`], and [`json_compilation_database`] formats those as
//! JSON for a file or any other writer. These three need the
//! `compile-commands` feature:
//!
//! ```no_run
//! # #[cfg(feature = "compile-commands")] {
//! use std::{env, fs, path::PathBuf, sync::Arc};
//! use cc::compile_commands::{json_compilation_database, CompileCommandCollector};
//!
//! let (collector, commands) = CompileCommandCollector::new();
//! let collector = Arc::new(collector);
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
//! let json = json_compilation_database(commands).to_string();
//! fs::write(out_dir.join("compile_commands.json"), json).unwrap();
//! # }
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
//! [`BuildMessageKind::CompileCommand`]: crate::BuildMessageKind::CompileCommand

// The collector's docs are only there with its feature.
#![cfg_attr(
    not(feature = "compile-commands"),
    allow(rustdoc::broken_intra_doc_links)
)]

use std::{ffi::OsStr, path::Path, process::Command};

#[cfg(feature = "compile-commands")]
mod collector;
#[cfg(feature = "compile-commands")]
pub use collector::{json_compilation_database, CompileCommandCollector, CompileCommands};

/// The command that compiles one object file: an entry of a
/// [JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html).
///
/// cc passes it as the `extra` of a
/// [`BuildMessageKind::CompileCommand`](crate::BuildMessageKind::CompileCommand)
/// message. See the [module docs](self) for what each field holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileCommand {
    directory: Box<Path>,
    file: Box<Path>,
    arguments: Box<[Box<OsStr>]>,
    output: Box<Path>,
}

impl CompileCommand {
    /// `cmd` compiles `src` into `dst` and runs in `directory`, which must be
    /// absolute.
    pub(crate) fn new(cmd: &Command, src: &Path, dst: &Path, directory: &Path) -> Self {
        // cc runs its compile commands in its own working directory.
        debug_assert!(cmd.get_current_dir().is_none());
        Self {
            directory: directory.into(),
            file: directory.join(src).into(),
            arguments: std::iter::once(cmd.get_program())
                .chain(cmd.get_args())
                .map(Box::from)
                .collect(),
            output: directory.join(dst).into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_command_has_no_spare_capacity() {
        // Each field is a boxed slice, a pointer and a length, without the
        // capacity a `PathBuf` or `Vec` would add.
        assert_eq!(
            std::mem::size_of::<CompileCommand>(),
            4 * std::mem::size_of::<Box<[u8]>>()
        );
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
}
