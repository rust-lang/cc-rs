//! Miscellaneous helpers for running commands

use std::{
    collections::hash_map,
    ffi::OsString,
    fmt::Display,
    fs::{self, File},
    hash::Hasher,
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::{Error, ErrorKind, Object};

#[derive(Clone, Debug)]
pub(crate) struct CargoOutput {
    pub(crate) metadata: bool,
    pub(crate) warnings: bool,
}

impl CargoOutput {
    pub(crate) const fn new() -> Self {
        Self {
            metadata: true,
            warnings: true,
        }
    }

    pub(crate) fn print_metadata(&self, s: &dyn Display) {
        if self.metadata {
            println!("{}", s);
        }
    }

    pub(crate) fn print_warning(&self, arg: &dyn Display) {
        if self.warnings {
            println!("cargo:warning={}", arg);
        }
    }

    pub(crate) fn print_thread(&self) -> Result<Option<PrintThread>, Error> {
        self.warnings.then(PrintThread::new).transpose()
    }
}

pub(crate) struct PrintThread {
    handle: Option<JoinHandle<()>>,
    pipe_writer: Option<File>,
}

impl PrintThread {
    pub(crate) fn new() -> Result<Self, Error> {
        let (pipe_reader, pipe_writer) = crate::os_pipe::pipe()?;

        // Capture the standard error coming from compilation, and write it out
        // with cargo:warning= prefixes. Note that this is a bit wonky to avoid
        // requiring the output to be UTF-8, we instead just ship bytes from one
        // location to another.
        let print = thread::spawn(move || {
            let mut stderr = BufReader::with_capacity(4096, pipe_reader);
            let mut line = Vec::with_capacity(20);
            let stdout = io::stdout();

            // read_until returns 0 on Eof
            while stderr.read_until(b'\n', &mut line).unwrap() != 0 {
                {
                    let mut stdout = stdout.lock();

                    stdout.write_all(b"cargo:warning=").unwrap();
                    stdout.write_all(&line).unwrap();
                    stdout.write_all(b"\n").unwrap();
                }

                // read_until does not clear the buffer
                line.clear();
            }
        });

        Ok(Self {
            handle: Some(print),
            pipe_writer: Some(pipe_writer),
        })
    }

    /// # Panics
    ///
    /// Will panic if the pipe writer has already been taken.
    pub(crate) fn take_pipe_writer(&mut self) -> File {
        self.pipe_writer.take().unwrap()
    }

    /// # Panics
    ///
    /// Will panic if the pipe writer has already been taken.
    pub(crate) fn clone_pipe_writer(&self) -> Result<File, Error> {
        self.try_clone_pipe_writer().map(Option::unwrap)
    }

    pub(crate) fn try_clone_pipe_writer(&self) -> Result<Option<File>, Error> {
        self.pipe_writer
            .as_ref()
            .map(File::try_clone)
            .transpose()
            .map_err(From::from)
    }
}

impl Drop for PrintThread {
    fn drop(&mut self) {
        // Drop pipe_writer first to avoid deadlock
        self.pipe_writer.take();

        self.handle.take().unwrap().join().unwrap();
    }
}

fn wait_on_child(cmd: &Command, program: &str, child: &mut Child) -> Result<(), Error> {
    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::ToolExecError,
                format!(
                    "Failed to wait on spawned child process, command {:?} with args {:?}: {}.",
                    cmd, program, e
                ),
            ));
        }
    };
    println!("{}", status);

    if status.success() {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::ToolExecError,
            format!(
                "Command {:?} with args {:?} did not execute successfully (status code {}).",
                cmd, program, status
            ),
        ))
    }
}

/// Find the destination object path for each file in the input source files,
/// and store them in the output Object.
pub(crate) fn objects_from_files(files: &[Arc<Path>], dst: &Path) -> Result<Vec<Object>, Error> {
    let mut objects = Vec::with_capacity(files.len());
    for file in files {
        let basename = file
            .file_name()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidArgument,
                    "No file_name for object file path!",
                )
            })?
            .to_string_lossy();
        let dirname = file
            .parent()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidArgument,
                    "No parent for object file path!",
                )
            })?
            .to_string_lossy();

        // Hash the dirname. This should prevent conflicts if we have multiple
        // object files with the same filename in different subfolders.
        let mut hasher = hash_map::DefaultHasher::new();
        hasher.write(dirname.to_string().as_bytes());
        let obj = dst
            .join(format!("{:016x}-{}", hasher.finish(), basename))
            .with_extension("o");

        match obj.parent() {
            Some(s) => fs::create_dir_all(s)?,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "dst is an invalid path with no parent",
                ));
            }
        };

        objects.push(Object::new(file.to_path_buf(), obj));
    }

    Ok(objects)
}

fn run_inner(cmd: &mut Command, program: &str, pipe_writer: Option<File>) -> Result<(), Error> {
    let mut child = spawn(cmd, program, pipe_writer)?;
    wait_on_child(cmd, program, &mut child)
}

pub(crate) fn run(
    cmd: &mut Command,
    program: &str,
    print: Option<&PrintThread>,
) -> Result<(), Error> {
    let pipe_writer = print.map(PrintThread::clone_pipe_writer).transpose()?;
    run_inner(cmd, program, pipe_writer)?;

    Ok(())
}

pub(crate) fn run_output(
    cmd: &mut Command,
    program: &str,
    cargo_output: &CargoOutput,
) -> Result<Vec<u8>, Error> {
    cmd.stdout(Stdio::piped());

    let mut print = cargo_output.print_thread()?;
    let mut child = spawn(
        cmd,
        program,
        print.as_mut().map(PrintThread::take_pipe_writer),
    )?;

    let mut stdout = vec![];
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();

    wait_on_child(cmd, program, &mut child)?;

    Ok(stdout)
}

pub(crate) fn spawn(
    cmd: &mut Command,
    program: &str,
    pipe_writer: Option<File>,
) -> Result<Child, Error> {
    struct ResetStderr<'cmd>(&'cmd mut Command);

    impl Drop for ResetStderr<'_> {
        fn drop(&mut self) {
            // Reset stderr to default to release pipe_writer so that print thread will
            // not block forever.
            self.0.stderr(Stdio::inherit());
        }
    }

    println!("running: {:?}", cmd);

    let cmd = ResetStderr(cmd);
    let child = cmd
        .0
        .stderr(pipe_writer.map_or_else(Stdio::null, Stdio::from))
        .spawn();
    match child {
        Ok(child) => Ok(child),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            let extra = if cfg!(windows) {
                " (see https://github.com/rust-lang/cc-rs#compile-time-requirements \
for help)"
            } else {
                ""
            };
            Err(Error::new(
                ErrorKind::ToolNotFound,
                format!("Failed to find tool. Is `{}` installed?{}", program, extra),
            ))
        }
        Err(e) => Err(Error::new(
            ErrorKind::ToolExecError,
            format!(
                "Command {:?} with args {:?} failed to start: {:?}",
                cmd.0, program, e
            ),
        )),
    }
}

pub(crate) fn command_add_output_file(
    cmd: &mut Command,
    dst: &Path,
    cuda: bool,
    msvc: bool,
    clang: bool,
    gnu: bool,
    is_asm: bool,
    is_arm: bool,
) {
    if msvc && !clang && !gnu && !cuda && !(is_asm && is_arm) {
        let mut s = OsString::from("-Fo");
        s.push(dst);
        cmd.arg(s);
    } else {
        cmd.arg("-o").arg(dst);
    }
}

#[cfg(feature = "parallel")]
pub(crate) fn try_wait_on_child(
    cmd: &Command,
    program: &str,
    child: &mut Child,
    stdout: &mut dyn io::Write,
) -> Result<Option<()>, Error> {
    match child.try_wait() {
        Ok(Some(status)) => {
            let _ = writeln!(stdout, "{}", status);

            if status.success() {
                Ok(Some(()))
            } else {
                Err(Error::new(
                    ErrorKind::ToolExecError,
                    format!(
                        "Command {:?} with args {:?} did not execute successfully (status code {}).",
                            cmd, program, status
                    ),
                ))
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(Error::new(
            ErrorKind::ToolExecError,
            format!(
                "Failed to wait on spawned child process, command {:?} with args {:?}: {}.",
                cmd, program, e
            ),
        )),
    }
}
