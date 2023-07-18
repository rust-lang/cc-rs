#![cfg_attr(test, allow(dead_code))]

/// This little test binary reads stdin and write what it reads to both
/// stdout and stderr. It depends on os_pipe's parent_* functions, and
/// we use it to test them.
use std::{env::args_os, fs::File, io, mem::ManuallyDrop, process::Command};

#[cfg(windows)]
use std::os::windows::prelude::*;

#[cfg(unix)]
use std::os::unix::prelude::*;

#[cfg(windows)]
fn dup(f: &dyn AsRawHandle) -> File {
    let handle = f.as_raw_handle();
    ManuallyDrop::new(unsafe { File::from_raw_handle(handle) })
        .try_clone()
        .unwrap()
}

#[cfg(unix)]
fn dup(f: &dyn AsRawFd) -> File {
    let handle = f.as_raw_fd();
    ManuallyDrop::new(unsafe { File::from_raw_fd(handle) })
        .try_clone()
        .unwrap()
}

fn main() {
    let stdin = dup(&io::stdin());
    let stdout = dup(&io::stdout());
    let stderr = dup(&io::stderr());

    let mut args = args_os();
    args.next().unwrap(); // Ignore args[0]
    let mut child = Command::new(args.next().unwrap()); // Run args[1]
    child.args(args); // Feed rest of the arg into the program

    // Swap stdout and stderr in the child. Set stdin too, just for testing,
    // though this should be the same as the default behavior.
    child.stdin(stdin);
    child.stdout(stderr);
    child.stderr(stdout);

    // Run the child. This method is kind of confusingly named...
    child.status().unwrap();
}
