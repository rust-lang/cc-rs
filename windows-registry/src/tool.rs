use std::{path::PathBuf, process::Command};

/// `Tool` found by `windows_registry`
#[derive(Clone, Debug)]
pub struct Tool {
    pub(crate) tool: PathBuf,
    pub(crate) is_clang_cl: bool,
}

impl Tool {
    /// Converts this compiler into a `Command` that's ready to be run.
    ///
    /// This is useful for when the compiler needs to be executed and the
    /// command returned will already have the initial arguments and environment
    /// variables configured.
    pub fn to_command(&self) -> Command {
        Command::new(&self.tool)
    }

    /// Check is the tool clang-cl related
    pub fn is_clang_cl(&self) -> bool {
        self.is_clang_cl
    }
}
