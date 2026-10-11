use std::{
    cell::Cell,
    io,
    os::unix::io::{AsRawFd, OwnedFd, RawFd},
    process::{Child, ChildStderr},
    time::Duration,
};

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::io::FromRawFd;

use crate::{cell_modify, cell_push};

#[derive(Default)]
pub(crate) struct Reactor {
    poll_fds: Cell<Vec<libc::pollfd>>,
    /// Set once `pidfd_open` turns out not to be available, such as on Linux
    /// before 5.3 or when blocked by seccomp, so it isn't tried again.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    no_pidfd: Cell<bool>,
}

pub(crate) struct Registration<'a> {
    reactor: &'a Reactor,
    fd: RawFd,
    /// Closed only after `fd` is removed from the reactor, in `drop`.
    _owned_fd: Option<OwnedFd>,
}

impl Reactor {
    pub(crate) fn register_stderr(&self, stderr: &ChildStderr) -> io::Result<Registration<'_>> {
        Ok(self.register_fd(stderr.as_raw_fd()))
    }

    pub(crate) fn register_fd(&self, fd: RawFd) -> Registration<'_> {
        cell_push(
            &self.poll_fds,
            libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            },
        );
        Registration {
            reactor: self,
            fd,
            _owned_fd: None,
        }
    }

    pub(crate) fn register_child(&self, child: &Child) -> io::Result<Option<Registration<'_>>> {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        if !self.no_pidfd.get() {
            // Not yet waited on, so the pid is still the child's.
            let ret = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_open,
                    libc::c_long::from(child.id() as libc::pid_t),
                    0 as libc::c_long,
                )
            };
            if ret >= 0 {
                // Always close-on-exec.
                let pidfd = unsafe { OwnedFd::from_raw_fd(ret as RawFd) };
                let mut registration = self.register_fd(pidfd.as_raw_fd());
                registration._owned_fd = Some(pidfd);
                return Ok(Some(registration));
            }
            let err = io::Error::last_os_error();
            if matches!(
                err.raw_os_error(),
                Some(libc::ENOSYS | libc::EPERM | libc::EACCES)
            ) {
                self.no_pidfd.set(true);
                return Ok(None);
            }

            return Err(err);
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        let _ = child;
        Ok(None)
    }

    pub(crate) fn wait_with_timeout(&self, timeout: Duration) -> io::Result<()> {
        let timeout = timeout.as_millis().try_into().unwrap();
        let result = cell_modify(&self.poll_fds, |poll_fds| {
            let ret = unsafe {
                libc::poll(
                    poll_fds.as_mut_ptr(),
                    poll_fds.len() as libc::nfds_t,
                    timeout,
                )
            };
            if ret < 0 {
                Err(io::Error::last_os_error())
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

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        let fd = self.fd;
        cell_modify(&self.reactor.poll_fds, |poll_fds| {
            if let Some(i) = poll_fds.iter().position(|pollfd| pollfd.fd == fd) {
                poll_fds.swap_remove(i);
            }
        });
    }
}
