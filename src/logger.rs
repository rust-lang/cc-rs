//! Structured access to the messages cc prints for Cargo.

use std::{
    any::Any,
    fmt,
    panic::{RefUnwindSafe, UnwindSafe},
    process::ExitStatus,
    sync::Arc,
};

/// Receives cc's messages, set with
/// [`Build::message_logger`](crate::Build::message_logger).
///
/// cc calls the logger on the thread that runs the `Build` method, also with
/// the `parallel` feature. Clones of a `Build` share the logger and may run on
/// other threads, so any synchronisation it needs is up to the logger.
pub trait BuildMessageLogger: Send + Sync + 'static {
    /// Called for each message.
    ///
    /// `kind` says what the message is about, `msg` holds its text, and
    /// `extra` holds more about it, depending on `kind`:
    ///
    /// - [`GeneralWarning`](BuildMessageKind::GeneralWarning): `()`
    /// - [`StderrForwarding`](BuildMessageKind::StderrForwarding) and
    ///   [`CommandFailed`](BuildMessageKind::CommandFailed): the
    ///   [`Command`](std::process::Command) that was run, read with
    ///   `extra.downcast_ref::<Command>()`
    ///
    /// The logger can't fail the build, but a panic in it reaches the caller
    /// of the `Build` method that was running.
    fn log(&self, kind: BuildMessageKind, msg: BuildMessage<'_>, extra: &dyn Any);
}

/// What a [`BuildMessage`] is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildMessageKind {
    /// Every warning cc prints itself, such as a failed compiler family
    /// detection or a setting that doesn't apply to the compiler. A failed
    /// command's error, which `parallel` also prints as a warning, arrives as
    /// [`CommandFailed`](BuildMessageKind::CommandFailed) instead.
    ///
    /// `extra` is `()`.
    GeneralWarning,
    /// Each line of stderr that cc forwards from a command it runs, such as
    /// the compiler or the archiver. Stderr that cc hides, such as that of
    /// flag support probes, isn't passed.
    ///
    /// A trailing `\r` is removed, and text that isn't UTF-8 is converted
    /// lossily. `extra` is the [`Command`](std::process::Command).
    StderrForwarding,
    /// A command cc runs exits unsuccessfully.
    ///
    /// The text is the error cc returns, and `extra` is the
    /// [`Command`](std::process::Command). Checks whose failure cc expects,
    /// such as flag support probes and the `ar` call with `D` that cc retries
    /// without it, are not reported. Neither are commands that can't be
    /// started, since they have no exit status.
    #[non_exhaustive]
    CommandFailed {
        /// `true` for detection commands that don't break the build: cc
        /// recovers when they fail, so the build can still succeed. Examples
        /// are the compiler's `--print-search-dirs` query,
        /// `xcrun --show-sdk-version` and compiler family detection. cc hides
        /// the output of family detection unless
        /// [`cargo_debug`](crate::Build::cargo_debug) is on, so its failures
        /// are only reported then, apart from the `--version` check it runs on
        /// clang. A failed family detection still arrives as a
        /// [`GeneralWarning`](BuildMessageKind::GeneralWarning).
        ///
        /// `false` for commands whose failure fails the build, such as the
        /// compiler, the archiver or `xcrun --show-sdk-path`.
        is_detection_cmd: bool,
        /// The status the command exited with.
        exit_status: ExitStatus,
    },
}

/// The text of a message passed to a [`BuildMessageLogger`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum BuildMessage<'a> {
    /// The message as a string.
    Str(&'a str),
}

impl fmt::Display for BuildMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(s) => f.write_str(s),
        }
    }
}

/// The logger of a `Build`, shared by its clones.
#[derive(Clone)]
pub(crate) struct Logger(pub(crate) Arc<dyn BuildMessageLogger>);

// cc only calls the logger and never relies on its state, so a panic can't
// leave cc with a broken one. This keeps `Build` `UnwindSafe` and
// `RefUnwindSafe`.
impl UnwindSafe for Logger {}
impl RefUnwindSafe for Logger {}

impl Logger {
    pub(crate) fn log(&self, kind: BuildMessageKind, msg: &str, extra: &dyn Any) {
        self.0.log(kind, BuildMessage::Str(msg), extra);
    }
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logger").finish_non_exhaustive()
    }
}
