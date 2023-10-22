use std::{
    ffi::{CString, OsString},
    io, ptr,
};

use crate::windows_sys::{
    OpenSemaphoreA, ReleaseSemaphore, WaitForSingleObject, FALSE, HANDLE, SEMAPHORE_MODIFY_STATE,
    THREAD_SYNCHRONIZE, WAIT_OBJECT_0,
};

pub(super) struct JobServerClient {
    sem: HANDLE,
}

unsafe impl Sync for JobServerClient {}
unsafe impl Send for JobServerClient {}

impl JobServerClient {
    pub(super) unsafe fn open(var: OsString) -> Option<Self> {
        let s = var
            .to_str()?
            .split_ascii_whitespace()
            .filter_map(|arg| {
                arg.strip_prefix("--jobserver-fds=")
                    .or_else(|| arg.strip_prefix("--jobserver-auth="))
            })
            .find(|s| !s.is_empty())?;

        let name = CString::new(s).ok()?;

        let sem = OpenSemaphoreA(
            THREAD_SYNCHRONIZE | SEMAPHORE_MODIFY_STATE,
            FALSE,
            name.as_bytes().as_ptr(),
        );
        if sem != ptr::null_mut() {
            Some(Self { sem })
        } else {
            None
        }
    }

    pub(super) fn try_acquire(&self) -> io::Result<Option<()>> {
        let r = unsafe { WaitForSingleObject(self.sem, 0) };
        if r == WAIT_OBJECT_0 {
            Ok(Some(()))
        } else if r == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(None)
        }
    }

    pub(super) fn release(&self) -> io::Result<()> {
        // SAFETY: ReleaseSemaphore will write to prev_count is it is Some
        // and release semaphore self.sem by 1.
        let r = unsafe { ReleaseSemaphore(self.sem, 1, ptr::null_mut()) };
        if r != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
