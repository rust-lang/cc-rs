use std::{
    cell::Cell,
    io,
    marker::PhantomData,
    os::unix::io::{AsRawFd, RawFd},
    process::ChildStderr,
    thread::sleep,
    time::Duration,
};

use crate::parallel::{cell_modify, cell_push};

#[derive(Default)]
pub(crate) struct Reactor {
    poll_fds: Cell<Vec<libc::pollfd>>,
}

pub(crate) struct Registration<'a> {
    reactor: &'a Reactor,
    fd: RawFd,
}

impl Reactor {
    pub(crate) fn register_stderr(&self, stderr: &ChildStderr) -> io::Result<Registration<'_>> {
        let fd = stderr.as_raw_fd();
        cell_push(
            &self.poll_fds,
            libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            },
        );
        Some(Registration { reactor: self, fd })
    }

    pub(crate) fn wait_with_timeout(&self, timeout: Duration) -> io::Result<()> {
        let timeout = timeout.as_millis().try_into().unwrap();
        let result = cell_modify(|poll_fds| {
            // An error, such as being interrupted by a signal, only means
            // waking up early: the runner checks every command anyway.
            let ret = unsafe {
                libc::poll(
                    poll_fds.as_mut_ptr(),
                    poll_fds.len() as libc::nfds_t,
                    timeout,
                )
            };
            if ret < 0 {
                Err(io::Error::last_os_error());
            } else {
                Ok(())
            }
        });
        match result {
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok(()),
            result => result,
        }
    }
}
