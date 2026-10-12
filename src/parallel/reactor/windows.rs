use std::{
    cell::Cell,
    io,
    marker::PhantomData,
    os::windows::io::{AsRawHandle, RawHandle},
    process::{Child, ChildStderr},
    thread::sleep,
    time::Duration,
};

use ::find_msvc_tools::windows_sys::{
    WaitForMultipleObjects, INFINITE, MAXIMUM_WAIT_OBJECTS, WAIT_FAILED,
};

use crate::{cell_modify, cell_push};

#[derive(Default)]
pub(crate) struct Reactor {
    /// Process handles, signaled once the process exits.
    handles: Cell<Vec<RawHandle>>,
}

/// Anonymous pipes can't be waited on, so stderr is only read on wake-ups
/// for other reasons, including its process exiting, and timeouts.
pub(crate) struct Registration<'a>(PhantomData<&'a Reactor>);

/// Keeps a child registered with the [`Reactor`] until dropped, which must
/// happen before the [`Child`] is dropped, as it uses its handle.
pub(crate) struct ChildRegistration<'a> {
    reactor: &'a Reactor,
    handle: RawHandle,
}

impl Reactor {
    pub(crate) fn register_stderr(&self, _stderr: &ChildStderr) -> io::Result<Registration<'_>> {
        Ok(Registration(PhantomData))
    }

    /// Wake [`Reactor::wait_with_timeout`] when `child` exits.
    pub(crate) fn register_child(&self, child: &Child) -> io::Result<ChildRegistration<'_>> {
        let handle = child.as_raw_handle();
        cell_push(&self.handles, handle);
        Ok(ChildRegistration {
            reactor: self,
            handle,
        })
    }

    pub(crate) fn wait_with_timeout(&self, timeout: Duration) -> io::Result<()> {
        // Never `INFINITE`, which would wait forever.
        let timeout = u32::try_from(timeout.as_millis())
            .unwrap_or(u32::MAX)
            .min(INFINITE - 1);
        cell_modify(&self.handles, |handles| {
            if handles.is_empty() {
                // Waiting on no handles is an error.
                sleep(Duration::from_millis(timeout.into()));
                return Ok(());
            }
            // Any beyond these are only noticed through other wake-ups, as
            // their order changes, or the timeout.
            let count = handles.len().min(MAXIMUM_WAIT_OBJECTS);
            let ret = unsafe { WaitForMultipleObjects(count as u32, handles.as_ptr(), 0, timeout) };
            if ret == WAIT_FAILED {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        })
    }
}

impl Drop for ChildRegistration<'_> {
    fn drop(&mut self) {
        let handle = self.handle;
        cell_modify(&self.reactor.handles, |handles| {
            if let Some(i) = handles.iter().position(|h| *h == handle) {
                handles.swap_remove(i);
            }
        });
    }
}
