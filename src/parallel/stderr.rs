/// Helpers functions for [ChildStderr].
use std::{convert::TryInto, process::ChildStderr};

use crate::{Error, ErrorKind};

#[cfg(all(not(unix), not(windows)))]
compile_error!("Only unix and windows support non-blocking pipes! For other OSes, disable the parallel feature.");

#[allow(unused_variables)]
pub fn set_non_blocking(stderr: &mut ChildStderr) -> Result<(), Error> {
    // On Unix, switch the pipe to non-blocking mode.
    // On Windows, we have a different way to be non-blocking.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = stderr.as_raw_fd();
        debug_assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_GETFL, 0) },
            0,
            "stderr should have no flags set"
        );

        if unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } != 0 {
            return Err(Error::new(
                ErrorKind::IOError,
                format!(
                    "Failed to set flags for child stderr: {}",
                    std::io::Error::last_os_error()
                ),
            ));
        }
    }

    Ok(())
}

pub fn bytes_available(stderr: &mut ChildStderr) -> Result<usize, Error> {
    let mut bytes_available = 0;
    #[cfg(windows)]
    {
        use crate::windows::windows_sys::PeekNamedPipe;
        use std::os::windows::io::AsRawHandle;
        use std::ptr::null_mut;
        if unsafe {
            PeekNamedPipe(
                stderr.as_raw_handle(),
                null_mut(),
                0,
                null_mut(),
                &mut bytes_available,
                null_mut(),
            )
        } == 0
        {
            return Err(Error::new(
                ErrorKind::IOError,
                format!(
                    "PeekNamedPipe failed with {}",
                    std::io::Error::last_os_error()
                ),
            ));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        if unsafe { libc::ioctl(stderr.as_raw_fd(), libc::FIONREAD, &mut bytes_available) } != 0 {
            return Err(Error::new(
                ErrorKind::IOError,
                format!("ioctl failed with {}", std::io::Error::last_os_error()),
            ));
        }
    }
    Ok(bytes_available.try_into().unwrap())
}
