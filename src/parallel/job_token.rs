use std::{mem::MaybeUninit, sync::Once};

use crate::Error;

pub(crate) struct JobToken();

impl Drop for JobToken {
    fn drop(&mut self) {
        match JobTokenServer::new() {
            JobTokenServer::Inherited(jobserver) => jobserver.release_token_raw(),
            JobTokenServer::InProcess(jobserver) => jobserver.release_token_raw(),
        }
    }
}

enum JobTokenServer {
    Inherited(inherited_jobserver::JobServer),
    InProcess(inprocess_jobserver::JobServer),
}

impl JobTokenServer {
    /// This function returns a static reference to the jobserver because
    ///  - creating a jobserver from env is a bit fd-unsafe (e.g. the fd might
    ///    be closed by other jobserver users in the process) and better do it
    ///    at the start of the program.
    ///  - in case a jobserver cannot be created from env (e.g. it's not
    ///    present), we will create a global in-process only jobserver
    ///    that has to be static so that it will be shared by all cc
    ///    compilation.
    fn new() -> &'static Self {
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
}

pub(crate) enum ActiveJobTokenServer {
    Inherited(inherited_jobserver::ActiveJobServer<'static>),
    InProcess(&'static inprocess_jobserver::JobServer),
}

impl ActiveJobTokenServer {
    pub(crate) fn new() -> Result<Self, Error> {
        match JobTokenServer::new() {
            JobTokenServer::Inherited(inherited_jobserver) => {
                inherited_jobserver.enter_active().map(Self::Inherited)
            }
            JobTokenServer::InProcess(inprocess_jobserver) => {
                Ok(Self::InProcess(inprocess_jobserver))
            }
        }
    }

    pub(crate) async fn acquire(&self) -> Result<JobToken, Error> {
        match &self {
            Self::Inherited(jobserver) => jobserver.acquire().await,
            Self::InProcess(jobserver) => Ok(jobserver.acquire().await),
        }
    }
}

mod inherited_jobserver {
    use super::JobToken;

    use crate::{parallel::async_executor::YieldOnce, Error, ErrorKind};

    use std::{
        io,
        sync::{
            atomic::{
                AtomicBool,
                Ordering::{AcqRel, Acquire},
            },
            mpsc,
        },
    };

    pub(super) struct JobServer {
        /// Implicit token for this process which is obtained and will be
        /// released in parent. Since JobTokens only give back what they got,
        /// there should be at most one global implicit token in the wild.
        ///
        /// Since Rust does not execute any `Drop` for global variables,
        /// we can't just put it back to jobserver and then re-acquire it at
        /// the end of the process.
        global_implicit_token: AtomicBool,
        inner: jobserver::Client,
    }

    impl JobServer {
        pub(super) unsafe fn from_env() -> Option<Self> {
            jobserver::Client::from_env().map(|inner| Self {
                inner,
                global_implicit_token: AtomicBool::new(true),
            })
        }

        pub(super) fn release_token_raw(&self) {
            // All tokens will be put back into the jobserver immediately
            // and they cannot be cached, since Rust does not call `Drop::drop`
            // on global variables.
            if self
                .global_implicit_token
                .compare_exchange(false, true, AcqRel, Acquire)
                .is_err()
            {
                // There's already a global implicit token, so this token must
                // be released back into jobserver
                let _ = self.inner.release_raw();
            }
        }

        pub(super) fn enter_active(&self) -> Result<ActiveJobServer<'_>, Error> {
            ActiveJobServer::new(self)
        }
    }

    pub(crate) struct ActiveJobServer<'a> {
        jobserver: &'a JobServer,
        helper_thread: jobserver::HelperThread,
        /// When rx is dropped, all the token stored within it will be dropped.
        rx: mpsc::Receiver<io::Result<jobserver::Acquired>>,
    }

    impl<'a> ActiveJobServer<'a> {
        fn new(jobserver: &'a JobServer) -> Result<Self, Error> {
            let (tx, rx) = mpsc::channel();

            Ok(Self {
                rx,
                helper_thread: jobserver.inner.clone().into_helper_thread(move |res| {
                    let _ = tx.send(res);
                })?,
                jobserver,
            })
        }

        pub(super) async fn acquire(&self) -> Result<JobToken, Error> {
            let mut has_requested_token = false;

            loop {
                if self.jobserver.global_implicit_token.swap(false, AcqRel) {
                    // fast path
                    break Ok(JobToken());
                }

                // Cold path, no global implicit token, obtain one
                match self.rx.try_recv() {
                    Ok(res) => {
                        let acquired = res?;
                        acquired.drop_without_releasing();
                        break Ok(JobToken());
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        break Err(Error::new(
                            ErrorKind::JobserverHelpThreadError,
                            "jobserver help thread has returned before ActiveJobServer is dropped",
                        ))
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        if !has_requested_token {
                            self.helper_thread.request_token();
                            has_requested_token = true;
                        }
                        YieldOnce::default().await
                    }
                }
            }
        }
    }
}

mod inprocess_jobserver {
    use super::JobToken;

    use crate::parallel::async_executor::YieldOnce;

    use std::{
        env::var,
        sync::atomic::{
            AtomicU32,
            Ordering::{AcqRel, Acquire},
        },
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
            // TODO: Use std::thread::available_parallelism as an upper bound
            // when MSRV is bumped.
            if let Ok(amt) = var("NUM_JOBS") {
                if let Ok(amt) = amt.parse() {
                    parallelism = amt;
                }
            }

            Self(AtomicU32::new(parallelism))
        }

        pub(super) async fn acquire(&self) -> JobToken {
            loop {
                let res = self
                    .0
                    .fetch_update(AcqRel, Acquire, |tokens| tokens.checked_sub(1));

                if res.is_ok() {
                    break JobToken();
                }

                YieldOnce::default().await
            }
        }

        pub(super) fn release_token_raw(&self) {
            self.0.fetch_add(1, AcqRel);
        }
    }
}
