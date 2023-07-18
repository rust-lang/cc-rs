//! Adapted from:
//!  - https://doc.rust-lang.org/src/std/sys/unix/pipe.rs.html
//!  - https://doc.rust-lang.org/src/std/sys/unix/fd.rs.html#385
//!  - https://github.com/rust-lang/rust/blob/master/library/std/src/sys/mod.rs#L57
//!  - https://github.com/oconnor663/os_pipe.rs
use std::{fmt, fs::File, io, process::Stdio};

macro_rules! impl_Read_by_forward {
    ($type:ty) => {
        impl io::Read for $type {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                (&self.0).read(buf)
            }

            fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
                (&self.0).read_vectored(bufs)
            }

            fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
                (&self.0).read_to_end(buf)
            }
            fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
                (&self.0).read_to_string(buf)
            }
            fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
                (&self.0).read_exact(buf)
            }
        }
    };
}

macro_rules! impl_Write_by_forward {
    ($type:ty) => {
        impl io::Write for $type {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                (&self.0).write(buf)
            }
            fn flush(&mut self) -> io::Result<()> {
                (&self.0).flush()
            }

            fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
                (&self.0).write_vectored(bufs)
            }
            fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
                (&self.0).write_all(buf)
            }
            fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
                (&self.0).write_fmt(fmt)
            }
        }
    };
}

/// The reading end of a pipe, returned by [`pipe`](fn.pipe.html).
///
/// `PipeReader` implements `Into<Stdio>`, so you can pass it as an argument to
/// `Command::stdin` to spawn a child process that reads from the pipe.
#[derive(Debug)]
pub struct PipeReader(File);

impl PipeReader {
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

impl_Read_by_forward!(PipeReader);
impl_Read_by_forward!(&PipeReader);

impl From<PipeReader> for Stdio {
    fn from(p: PipeReader) -> Stdio {
        p.0.into()
    }
}

/// The writing end of a pipe, returned by [`pipe`](fn.pipe.html).
///
/// `PipeWriter` implements `Into<Stdio>`, so you can pass it as an argument to
/// `Command::stdout` or `Command::stderr` to spawn a child process that writes
/// to the pipe.
#[derive(Debug)]
pub struct PipeWriter(File);

impl PipeWriter {
    pub fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Self)
    }
}

impl_Write_by_forward!(PipeWriter);
impl_Write_by_forward!(&PipeWriter);

impl From<PipeWriter> for Stdio {
    fn from(p: PipeWriter) -> Stdio {
        p.0.into()
    }
}

/// Open a new pipe and return a [`PipeReader`] and [`PipeWriter`] pair.
///
/// This corresponds to the `pipe2` library call on Posix and the
/// `CreatePipe` library call on Windows (though these implementation
/// details might change). These pipes are non-inheritable, so new child
/// processes won't receive a copy of them unless they're explicitly
/// passed as stdin/stdout/stderr.
///
/// [`PipeReader`]: struct.PipeReader.html
/// [`PipeWriter`]: struct.PipeWriter.html
pub fn pipe() -> io::Result<(PipeReader, PipeWriter)> {
    sys::pipe().map(|(r, w)| (PipeReader(r), PipeWriter(w)))
}

#[cfg(unix)]
#[path = "os_pipe/unix.rs"]
mod sys;

#[cfg(windows)]
#[path = "os_pipe/windows.rs"]
mod sys;

#[cfg(all(not(unix), not(windows)))]
compile_error!("Only unix and windows support os_pipe!");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env::{self, consts::EXE_EXTENSION},
        io::prelude::*,
        path::PathBuf,
        process::Command,
        thread,
    };

    fn path_to_exe(name: &str) -> PathBuf {
        let mut p = env::current_exe().unwrap();
        p.pop();
        if p.ends_with("deps") {
            p.pop();
        }
        p.push(name);
        p.set_extension(EXE_EXTENSION);

        p
    }

    #[test]
    fn test_pipe_some_data() {
        let (mut reader, mut writer) = pipe().unwrap();
        // A small write won't fill the pipe buffer, so it won't block this thread.
        writer.write_all(b"some stuff").unwrap();
        drop(writer);
        let mut out = String::new();
        reader.read_to_string(&mut out).unwrap();
        assert_eq!(out, "some stuff");
    }

    #[test]
    fn test_pipe_some_data_with_refs() {
        // As with `File`, there's a second set of impls for shared
        // refs. Test those.
        let (reader, writer) = pipe().unwrap();
        let mut reader_ref = &reader;
        {
            let mut writer_ref = &writer;
            // A small write won't fill the pipe buffer, so it won't block this thread.
            writer_ref.write_all(b"some stuff").unwrap();
        }
        drop(writer);
        let mut out = String::new();
        reader_ref.read_to_string(&mut out).unwrap();
        assert_eq!(out, "some stuff");
    }

    #[test]
    fn test_pipe_no_data() {
        let (mut reader, writer) = pipe().unwrap();
        drop(writer);
        let mut out = String::new();
        reader.read_to_string(&mut out).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_pipe_a_megabyte_of_data_from_another_thread() {
        let data = vec![0xff; 1_000_000];
        let data_copy = data.clone();
        let (mut reader, mut writer) = pipe().unwrap();
        let joiner = thread::spawn(move || {
            writer.write_all(&data_copy).unwrap();
            // This drop happens automatically, so writing it out here is mostly
            // just for clarity. For what it's worth, it also guards against
            // accidentally forgetting to drop if we switch to scoped threads or
            // something like that and change this to a non-moving closure. The
            // explicit drop forces `writer` to move.
            drop(writer);
        });
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        joiner.join().unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn test_pipes_are_not_inheritable() {
        // Create pipes for a child process.
        let (input_reader, mut input_writer) = pipe().unwrap();
        let (mut output_reader, output_writer) = pipe().unwrap();

        // Create a bunch of duplicated copies, which we'll close later. This
        // tests that duplication preserves non-inheritability.
        let ir_dup = input_reader.try_clone().unwrap();
        let iw_dup = input_writer.try_clone().unwrap();
        let or_dup = output_reader.try_clone().unwrap();
        let ow_dup = output_writer.try_clone().unwrap();

        // Spawn the child. Note that this temporary Command object takes
        // ownership of our copies of the child's stdin and stdout, and then
        // closes them immediately when it drops. That stops us from blocking
        // our own read below. We use our own simple implementation of cat for
        // compatibility with Windows.
        let mut child = Command::new(path_to_exe("cat"))
            .stdin(input_reader)
            .stdout(output_writer)
            .spawn()
            .unwrap();

        // Drop all the dups now that the child is spawned.
        drop(ir_dup);
        drop(iw_dup);
        drop(or_dup);
        drop(ow_dup);

        // Write to the child's stdin. This is a small write, so it shouldn't
        // block.
        input_writer.write_all(b"hello").unwrap();
        drop(input_writer);

        // Read from the child's stdout. If this child has accidentally
        // inherited the write end of its own stdin, then it will never exit,
        // and this read will block forever. That's what this test is all
        // about.
        let mut output = Vec::new();
        output_reader.read_to_end(&mut output).unwrap();
        child.wait().unwrap();

        // Confirm that we got the right bytes.
        assert_eq!(b"hello", &*output);
    }

    #[test]
    fn test_parent_handles() {
        // This test invokes the `swap` test program, which uses parent_stdout() and
        // parent_stderr() to swap the outputs for another child that it spawns.

        // Create pipes for a child process.
        let (reader, mut writer) = pipe().unwrap();

        // Write input. This shouldn't block because it's small. Then close the write end, or else
        // the child will hang.
        writer.write_all(b"quack").unwrap();
        drop(writer);

        // Use `swap` to run `cat_both`. `cat_both will read "quack" from stdin
        // and write it to stdout and stderr with different tags. But because we
        // run it inside `swap`, the tags in the output should be backwards.
        let output = Command::new(path_to_exe("swap"))
            .arg(path_to_exe("cat_both"))
            .stdin(reader)
            .output()
            .unwrap();

        // Check for a clean exit.
        assert!(
            output.status.success(),
            "child process returned {:#?}",
            output
        );

        // Confirm that we got the right bytes.
        assert_eq!(b"stderr: quack", &*output.stdout);
        assert_eq!(b"stdout: quack", &*output.stderr);
    }

    #[test]
    fn test_try_clone() {
        let (reader, writer) = pipe().unwrap();
        let mut reader_clone = reader.try_clone().unwrap();
        let mut writer_clone = writer.try_clone().unwrap();
        // A small write won't fill the pipe buffer, so it won't block this thread.
        writer_clone.write_all(b"some stuff").unwrap();
        drop(writer);
        drop(writer_clone);
        let mut out = String::new();
        reader_clone.read_to_string(&mut out).unwrap();
        assert_eq!(out, "some stuff");
    }

    #[test]
    fn test_debug() {
        let (reader, writer) = pipe().unwrap();
        format!("{:?} {:?}", reader, writer);
    }
}
