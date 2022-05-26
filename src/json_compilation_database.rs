use std::path::PathBuf;
use std::process::Command;
#[cfg(feature = "compile_commands")]
use tinyjson::JsonValue;

/// An entry for creating a [JSON Compilation Database](https://clang.llvm.org/docs/JSONCompilationDatabase.html).
pub struct CompileCommand {
    directory: PathBuf,
    arguments: Vec<String>,
    file: PathBuf,
    output: PathBuf,
}

impl CompileCommand {
    #[cfg(feature = "compile_commands")]
    pub(crate) fn new(cmd: &Command, src: PathBuf, output: PathBuf) -> Self {
        let mut arguments = Vec::with_capacity(cmd.get_args().len() + 1);

        let program = String::from(cmd.get_program().to_str().unwrap());
        arguments.push(
            crate::which(&program)
                .map(|p| p.to_string_lossy().into_owned())
                .map(|p| p.to_string())
                .unwrap_or(program),
        );
        arguments.extend(
            cmd.get_args()
                .flat_map(std::ffi::OsStr::to_str)
                .map(String::from),
        );

        Self {
            // TODO: is the assumption correct?
            directory: std::env::current_dir().unwrap(),
            arguments,
            file: src,
            output,
        }
    }

    /// This is a dummy implementation when `Command::get_args` is unavailable (e.g. MSRV or older
    /// Rust versions)
    #[cfg(not(feature = "compile_commands"))]
    pub(crate) fn new(_cmd: &Command, src: PathBuf, output: PathBuf) -> Self {
        Self {
            // TODO: is the assumption correct?
            directory: std::env::current_dir().unwrap(),
            arguments: Vec::new(),
            file: src,
            output,
        }
    }

    /// The working directory of the compilation. All paths specified in the command or file fields
    /// must be either absolute or relative to this directory.
    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    /// The name of the output created by this compilation step. This field is optional. It can be
    /// used to distinguish different processing modes of the same input file.
    pub fn output(&self) -> &PathBuf {
        &self.output
    }

    /// The main translation unit source processed by this compilation step. This is used by tools
    /// as the key into the compilation database. There can be multiple command objects for the
    /// same file, for example if the same source file is compiled with different configurations.
    pub fn file(&self) -> &PathBuf {
        &self.file
    }

    /// The compile command argv as list of strings. This should run the compilation step for the
    /// translation unit file. arguments[0] should be the executable name, such as clang++.
    /// Arguments should not be escaped, but ready to pass to execvp().
    pub fn arguments(&self) -> &Vec<String> {
        &self.arguments
    }
}

/// Stores the provided list of [compile commands](crate::CompileCommand) as [JSON
/// Compilation Database](https://clang.llvm.org/docs/JSONCompilationDatabase.html).
#[cfg(feature = "compile_commands")]
pub fn store_json_compilation_database<'a, C, P>(commands: C, path: P)
where
    C: IntoIterator<Item = &'a CompileCommand>,
    P: AsRef<std::path::Path>,
{
    let db = JsonValue::Array(
        commands
            .into_iter()
            .map(|command| command.into())
            .collect::<Vec<JsonValue>>(),
    );

    std::fs::write(path, db.stringify().unwrap()).unwrap();
}

#[cfg(feature = "compile_commands")]
impl<'a> std::convert::From<&CompileCommand> for JsonValue {
    fn from(compile_command: &CompileCommand) -> Self {
        use std::collections::HashMap;
        JsonValue::Object(HashMap::from([
            (
                String::from("directory"),
                JsonValue::String(compile_command.directory.to_string_lossy().to_string()),
            ),
            (
                String::from("file"),
                JsonValue::String(compile_command.file.to_string_lossy().to_string()),
            ),
            (
                String::from("output"),
                JsonValue::String(compile_command.output.to_string_lossy().to_string()),
            ),
            (
                String::from("arguments"),
                JsonValue::Array(
                    compile_command
                        .arguments
                        .iter()
                        .map(|arg| JsonValue::String(arg.to_string()))
                        .collect::<Vec<_>>(),
                ),
            ),
        ]))
    }
}
