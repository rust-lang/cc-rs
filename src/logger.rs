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
pub trait BuildMessageLogger: Send + Sync + 'static {
    /// Called for each message.
    ///
    /// `extra` holds more about the message, depending on `kind`:
    ///
    /// - [`GeneralWarning`](BuildMessageKind::GeneralWarning): `()`
    /// - [`StderrForwarding`](BuildMessageKind::StderrForwarding) and
    ///   [`CommandFailed`](BuildMessageKind::CommandFailed): the
    ///   [`Command`](std::process::Command) that was run, read with
    ///   `extra.downcast_ref::<Command>()`
    fn log(&self, kind: BuildMessageKind, msg: BuildMessage<'_>, extra: &dyn Any);
}

/// What a [`BuildMessage`] is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildMessageKind {
    /// A warning from cc itself.
    GeneralWarning,
    /// One line of stderr from a command cc ran, such as the compiler.
    StderrForwarding,
    /// A command cc ran exited with this unsuccessful status.
    ///
    /// cc recovers from some failed commands, such as a query for the
    /// compiler's search paths, so the build may still succeed.
    CommandFailed(ExitStatus),
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
