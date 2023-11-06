use std::{
    ffi::{CString, OsString},
    io, ptr,
};

use crate::windows_sys::{
    OpenSemaphoreA, ReleaseSemaphore, WaitForSingleObject, FALSE, HANDLE, SEMAPHORE_MODIFY_STATE,
    THREAD_SYNCHRONIZE, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};

const WAIT_ABANDOEND_ERR_MSG: &str = r#" The specified object is a mutex object that was not released by the thread that owned the mutex object before the owning thread terminated. Ownership of the mutex object is granted to the calling thread and the mutex state is set to nonsignaled.

If the mutex was protecting persistent state information, you should check it for consistency."#;

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
            .filter_map(|arg| arg.strip_prefix("--jobserver-auth="))
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
        match unsafe { WaitForSingleObject(self.sem, 0) } {
            WAIT_OBJECT_0 => Ok(Some(())),
            WAIT_TIMEOUT => Ok(None),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            WAIT_ABANDONED => Err(io::Error::new(io::ErrorKind::Other, WAIT_ABANDOEND_ERR_MSG)),
            _ => unreachable!("Unexpected return value from WaitForSingleObject"),
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
