use std::{mem::MaybeUninit, sync::Once};

#[cfg(unix)]
#[path = "job_token/unix.rs"]
mod sys;

#[cfg(windows)]
#[path = "job_token/windows.rs"]
mod sys;

pub(super) enum JobToken {
    Inherited(inherited_jobserver::JobToken),
    InProcess(inprocess_jobserver::JobToken),
}

pub(super) enum JobTokenServer {
    Inherited(inherited_jobserver::JobServer),
    InProcess(inprocess_jobserver::JobServer),
}

impl JobTokenServer {
    pub(crate) fn new() -> &'static Self {
        static INIT: Once = Once::new();
        static mut JOBSERVER: MaybeUninit<JobTokenServer> = MaybeUninit::uninit();

        unsafe {
            INIT.call_once(|| {
                let server = inherited_jobserver::JobServer::from_env()
                    .map(Self::Inherited)
                    .unwrap_or_else(|| Self::InProcess(inprocess_jobserver::JobServer::new()));
                JOBSERVER = MaybeUninit::new(server);
            });
            // TODO: Poor man's assume_init_ref, as that'd require a MSRV of 1.55.
            &*JOBSERVER.as_ptr()
        }
    }

    pub(crate) fn try_acquire(&'static self) -> Result<Option<JobToken>, crate::Error> {
        match self {
            Self::Inherited(jobserver) => jobserver
                .try_acquire()
                .map(|option| option.map(JobToken::Inherited)),
            Self::InProcess(jobserver) => Ok(jobserver.try_acquire().map(JobToken::InProcess)),
        }
    }
}

mod inherited_jobserver {
    use super::sys;

    use std::{
        env::var_os,
        sync::atomic::{AtomicBool, Ordering::Relaxed},
    };

    pub(crate) struct JobServer {
        /// Implicit token for this process which is obtained and will be
        /// released in parent. Since JobTokens only give back what they got,
        /// there should be at most one global implicit token in the wild.
        ///
        /// Since Rust does not execute any `Drop` for global variables,
        /// we can't just put it back to jobserver and then re-acquire it at
        /// the end of the process.
        global_implicit_token: AtomicBool,
        inner: sys::JobServerClient,
    }

    impl JobServer {
        pub(super) unsafe fn from_env() -> Option<Self> {
            let var = var_os("CARGO_MAKEFLAGS")
                .or_else(|| var_os("MAKEFLAGS"))
                .or_else(|| var_os("MFLAGS"))?;

            let inner = sys::JobServerClient::open(var)?;

            Some(Self {
                inner,
                global_implicit_token: AtomicBool::new(true),
            })
        }

        pub(super) fn try_acquire(&'static self) -> Result<Option<JobToken>, crate::Error> {
            if !self.global_implicit_token.swap(false, Relaxed) {
                // Cold path, no global implicit token, obtain one
                if self.inner.try_acquire()?.is_none() {
                    return Ok(None);
                }
            }
            Ok(Some(JobToken { jobserver: self }))
        }
    }

    /// A thin wrapper around jobserver Client.
    /// It would be perfectly fine to just use jobserver Client, but we also want to reuse
    /// our own implicit token assigned for this build script. This struct manages that and
    /// gives out tokens without exposing whether they're implicit tokens or tokens from jobserver.
    /// Furthermore, instead of giving up job tokens, it keeps them around
    /// for reuse if we know we're going to request another token after freeing the current one.
    pub(crate) struct JobToken {
        jobserver: &'static JobServer,
    }

    impl Drop for JobToken {
        fn drop(&mut self) {
            let jobserver = self.jobserver;
            if jobserver
                .global_implicit_token
                .compare_exchange(false, true, Relaxed, Relaxed)
                .is_err()
            {
                // There's already a global implicit token, so this token must
                // be released back into jobserver
                let _ = jobserver.inner.release();
            }
        }
    }
}

mod inprocess_jobserver {
    use std::{
        env::var,
        sync::atomic::{AtomicU32, Ordering::Relaxed},
    };

    pub(crate) struct JobServer(AtomicU32);

    impl JobServer {
        pub(super) fn new() -> Self {
            // Use `NUM_JOBS` if set (it's configured by Cargo) and otherwise
            // just fall back to a semi-reasonable number.
            //
            // Note that we could use `num_cpus` here but it's an extra
            // dependency that will almost never be used, so
            // it's generally not too worth it.
            let mut parallelism = 4;
            if let Ok(amt) = var("NUM_JOBS") {
                if let Ok(amt) = amt.parse() {
                    parallelism = amt;
                }
            }

            Self(AtomicU32::new(parallelism))
        }

        pub(super) fn try_acquire(&'static self) -> Option<JobToken> {
            let res = self.0.fetch_update(Relaxed, Relaxed, |tokens| {
                if tokens > 0 {
                    Some(tokens - 1)
                } else {
                    None
                }
            });

            if res.is_ok() {
                Some(JobToken(self))
            } else {
                None
            }
        }
    }

    pub(crate) struct JobToken(&'static JobServer);

    impl Drop for JobToken {
        fn drop(&mut self) {
            self.0 .0.fetch_add(1, Relaxed);
        }
    }
}
