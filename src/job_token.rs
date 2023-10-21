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

impl JobToken {
    /// Ensure that this token is not put back into queue once it's dropped.
    /// This also leads to releasing it sooner for other processes to use,
    /// which is a correct thing to do once it is known that there won't be
    /// any more token acquisitions.
    pub(super) fn forget(&mut self) {
        if let Self::Inherited(inherited_jobtoken) = self {
            inherited_jobtoken.forget();
        }
    }
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

    pub(crate) fn try_acquire(&self) -> Result<Option<JobToken>, crate::Error> {
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
        sync::mpsc::{self, Receiver, Sender},
    };

    pub(super) struct JobServer {
        inner: sys::JobServerClient,
        tx: Sender<Result<(), crate::Error>>,
        rx: Receiver<Result<(), crate::Error>>,
    }

    impl JobServer {
        pub(super) unsafe fn from_env() -> Option<Self> {
            let var = var_os("CARGO_MAKEFLAGS")
                .or_else(|| var_os("MAKEFLAGS"))
                .or_else(|| var_os("MFLAGS"))?;

            let inner = sys::JobServerClient::open(var)?;

            let (tx, rx) = mpsc::channel();
            // Push the implicit token. Since JobTokens only give back what they got,
            // there should be at most one global implicit token in the wild.
            tx.send(Ok(())).unwrap();

            Some(Self { inner, tx, rx })
        }

        pub(super) fn try_acquire(&self) -> Result<Option<JobToken>, crate::Error> {
            if let Ok(token) = self.rx.try_recv() {
                // Opportunistically check if there's a token that can be reused.
                token?
            } else {
                // Cold path, request a token
                if self.inner.try_acquire()?.is_none() {
                    return Ok(None);
                }
            };
            Ok(Some(JobToken {
                pool: Some(self.tx.clone()),
                jobserver: self,
            }))
        }
    }

    /// A thin wrapper around jobserver Client.
    /// It would be perfectly fine to just use jobserver Client, but we also want to reuse
    /// our own implicit token assigned for this build script. This struct manages that and
    /// gives out tokens without exposing whether they're implicit tokens or tokens from jobserver.
    /// Furthermore, instead of giving up job tokens, it keeps them around
    /// for reuse if we know we're going to request another token after freeing the current one.
    pub(super) struct JobToken {
        /// A pool to which `token` should be returned. `pool` is optional, as one might want to release a token straight away instead
        /// of storing it back in the pool - see [`Self::forget()`] function for that.
        pool: Option<Sender<Result<(), crate::Error>>>,
        jobserver: &'static JobServer,
    }

    impl Drop for JobToken {
        fn drop(&mut self) {
            if let Some(pool) = &self.pool {
                // Always send back an Ok() variant as we know that the acquisition for this token has succeeded.
                let _ = pool.send(Ok(()));
            } else {
                let _ = self.jobserver.inner.release();
            }
        }
    }

    impl JobToken {
        /// Ensure that this token is not put back into queue once it's dropped.
        /// This also leads to releasing it sooner for other processes to use,
        /// which is a correct thing to do once it is known that there won't be
        /// any more token acquisitions.
        pub(super) fn forget(&mut self) {
            self.pool.take();
        }
    }
}

mod inprocess_jobserver {
    use std::{
        env::var,
        sync::atomic::{AtomicU32, Ordering::Relaxed},
    };

    pub(super) struct JobServer(AtomicU32);

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

        pub(super) fn try_acquire(&self) -> Option<JobToken> {
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

    pub(super) struct JobToken(&'static JobServer);

    impl Drop for JobToken {
        fn drop(&mut self) {
            self.0 .0.fetch_add(1, Relaxed);
        }
    }
}
