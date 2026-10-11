use core::marker::PhantomData;

use crate::{parallel::reactor::Reactor, utilities::OnceLock, Error};

pub(crate) struct JobToken(PhantomData<()>);

impl JobToken {
    fn new() -> Self {
        Self(PhantomData)
    }
}

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
        // TODO: Replace with a OnceLock once MSRV is 1.70
        static JOBSERVER: OnceLock<JobTokenServer> = OnceLock::new();

        JOBSERVER.get_or_init(|| {
            unsafe { inherited_jobserver::JobServer::from_env() }
                .map(Self::Inherited)
                .unwrap_or_else(|| Self::InProcess(inprocess_jobserver::JobServer::new()))
        })
    }
}

pub(crate) enum ActiveJobTokenServer {
    Inherited(inherited_jobserver::ActiveJobServer<'static>),
    InProcess(&'static inprocess_jobserver::JobServer),
}

impl ActiveJobTokenServer {
    pub(crate) fn new() -> Self {
        match JobTokenServer::new() {
            JobTokenServer::Inherited(inherited_jobserver) => {
                Self::Inherited(inherited_jobserver.enter_active())
            }
            JobTokenServer::InProcess(inprocess_jobserver) => Self::InProcess(inprocess_jobserver),
        }
    }

    pub(crate) async fn acquire(&mut self, reactor: &Reactor) -> Result<JobToken, Error> {
        match self {
            Self::Inherited(jobserver) => jobserver.acquire(reactor).await,
            Self::InProcess(jobserver) => Ok(jobserver.acquire().await),
        }
    }
}

mod inherited_jobserver {
    use super::JobToken;

    use crate::{
        parallel::{async_executor::YieldOnce, reactor::Reactor},
        Error, ErrorKind,
    };

    #[cfg(unix)]
    use std::os::unix::io::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
    use std::{
        io, mem,
        sync::{mpsc, Mutex, MutexGuard, PoisonError},
    };

    pub(super) struct JobServer {
        /// Implicit token for this process which is obtained and will be
        /// released in parent. Since `JobTokens` only give back what they got,
        /// there should be at most one global implicit token in the wild.
        ///
        /// Since Rust does not execute any `Drop` for global variables,
        /// we can't just put it back to jobserver and then re-acquire it at
        /// the end of the process.
        ///
        /// Use `Mutex` to avoid race between acquire and release.
        /// If an `AtomicBool` is used, then it's possible for:
        ///  - `release_token_raw`: Tries to set `global_implicit_token` to true, but it is already
        ///    set  to `true`, continue to release it to jobserver
        ///  - `acquire` takes the global implicit token, set `global_implicit_token` to false
        ///  - `release_token_raw` now writes the token back into the jobserver, while
        ///    `global_implicit_token` is `false`
        ///
        /// If the program exits here, then cc effectively increases parallelism by one, which is
        /// incorrect, hence we use a `Mutex` here.
        global_implicit_token: Mutex<bool>,
        inner: jobserver::Client,
        #[cfg(unix)]
        fd: Option<OwnedFd>,
    }

    impl JobServer {
        pub(super) unsafe fn from_env() -> Option<Self> {
            jobserver::Client::from_env().map(|inner| Self {
                #[cfg(unix)]
                fd: Self::jobserver_fd(&inner),
                inner,
                global_implicit_token: Mutex::new(true),
            })
        }

        /// `jobserver::Client` doesn't expose its pipe or fifo, so take it from
        /// the `MAKEFLAGS` it passes to child processes, which is what they use.
        #[cfg(unix)]
        fn jobserver_fd(client: &jobserver::Client) -> Option<OwnedFd> {
            use std::{fs::OpenOptions, process::Command};

            let mut cmd = Command::new("");
            client.configure(&mut cmd);
            let (_, flags) = cmd.get_envs().find(|(key, _)| *key == "CARGO_MAKEFLAGS")?;
            // Last in the flags, so a fifo path may contain spaces.
            let (_, auth) = flags?.to_str()?.split_once("--jobserver-auth=")?;

            if let Some(path) = auth.strip_prefix("fifo:") {
                // Read and write, as opening a fifo read-only blocks until it has
                // a writer.
                let fifo = OpenOptions::new().read(true).write(true).open(path);
                Some(fifo.ok()?.into())
            } else {
                let (read, _) = auth.split_once(',')?;
                let read = read.parse::<RawFd>().ok().filter(|fd| *fd >= 0)?;
                // `from_env` has just opened the client from this fd, and left it
                // open.
                unsafe { BorrowedFd::borrow_raw(read) }
                    .try_clone_to_owned()
                    .ok()
            }
        }

        fn get_global_implicit_token(&self) -> MutexGuard<'_, bool> {
            self.global_implicit_token
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        }

        /// All tokens except for the global implicit token will be put back into the jobserver
        /// immediately and they cannot be cached, since Rust does not call `Drop::drop` on
        /// global variables.
        pub(super) fn release_token_raw(&self) {
            let mut global_implicit_token = self.get_global_implicit_token();

            if *global_implicit_token {
                // There's already a global implicit token, so this token must
                // be released back into jobserver.
                //
                // `release_raw` should not block
                let _ = self.inner.release_raw();
            } else {
                *global_implicit_token = true;
            }
        }

        pub(super) fn enter_active(&self) -> ActiveJobServer<'_> {
            ActiveJobServer {
                jobserver: self,
                helper_thread: None,
            }
        }
    }

    struct HelperThread {
        inner: jobserver::HelperThread,
        /// When rx is dropped, all the token stored within it will be dropped.
        rx: mpsc::Receiver<io::Result<jobserver::Acquired>>,
        /// Whether a token was requested and is yet to be received.
        ///
        /// Kept across `acquire`s: one that ends up with another token leaves
        /// its request pending, and requesting again would make the helper
        /// thread take more tokens than are ever used.
        has_requested_token: bool,
    }

    impl HelperThread {
        fn new(jobserver: &JobServer) -> Result<Self, Error> {
            let (tx, rx) = mpsc::channel();

            Ok(Self {
                rx,
                inner: jobserver.inner.clone().into_helper_thread(move |res| {
                    let _ = tx.send(res);
                })?,
                has_requested_token: false,
            })
        }

        fn try_recv(&mut self) -> Result<Option<JobToken>, Error> {
            match self.rx.try_recv() {
                Ok(res) => {
                    self.has_requested_token = false;
                    res?.drop_without_releasing();
                    Ok(Some(JobToken::new()))
                }
                Err(mpsc::TryRecvError::Disconnected) => Err(Error::new(
                    ErrorKind::JobserverHelpThreadError,
                    "jobserver help thread has returned before ActiveJobServer is dropped",
                )),
                Err(mpsc::TryRecvError::Empty) => Ok(None),
            }
        }

        /// At most one at a time, see `has_requested_token`.
        fn request_token(&mut self) {
            if !self.has_requested_token {
                self.inner.request_token();
                self.has_requested_token = true;
            }
        }
    }

    pub(crate) struct ActiveJobServer<'a> {
        jobserver: &'a JobServer,
        helper_thread: Option<HelperThread>,
    }

    impl ActiveJobServer<'_> {
        pub(super) async fn acquire(&mut self, _reactor: &Reactor) -> Result<JobToken, Error> {
            // Kept while waiting, dropped once a token is acquired: a
            // jobserver with tokens to spare would wake every wait at once.
            #[cfg(unix)]
            let mut _registration = None;

            loop {
                // Fast path
                if mem::replace(&mut *self.jobserver.get_global_implicit_token(), false) {
                    break Ok(JobToken::new());
                }

                match self.jobserver.inner.try_acquire() {
                    Ok(Some(acquired)) => {
                        acquired.drop_without_releasing();
                        break Ok(JobToken::new());
                    }
                    Ok(None) => {
                        // Other processes, such as rustc, other build
                        // scripts or make, put the tokens they release back
                        // into the jobserver.
                        #[cfg(unix)]
                        if _registration.is_none() {
                            _registration = self
                                .jobserver
                                .fd
                                .as_ref()
                                .map(|fd| _reactor.register_fd(fd.as_raw_fd()));
                        }
                        YieldOnce::default().await
                    }
                    Err(err) if err.kind() == io::ErrorKind::Unsupported => {
                        // Fallback to creating a help thread with blocking acquire
                        let helper_thread = if let Some(thread) = self.helper_thread.as_mut() {
                            thread
                        } else {
                            self.helper_thread
                                .insert(HelperThread::new(self.jobserver)?)
                        };

                        if let Some(token) = helper_thread.try_recv()? {
                            break Ok(token);
                        }

                        helper_thread.request_token();
                        YieldOnce::default().await
                    }
                    Err(err) => break Err(err.into()),
                }
            }
        }
    }
}

mod inprocess_jobserver {
    use super::JobToken;

    use crate::{parallel::async_executor::YieldOnce, utilities::cargo_env_var_os};

    use core::sync::atomic::{
        AtomicU32,
        Ordering::{AcqRel, Acquire},
    };

    pub(crate) struct JobServer(AtomicU32);

    impl JobServer {
        pub(super) fn new() -> Self {
            // Use `NUM_JOBS` if set (it's configured by Cargo) and otherwise
            // just fall back to the number of cores on the local machine, or a reasonable
            // default if that cannot be determined.

            let parallelism = cargo_env_var_os("NUM_JOBS")
                .and_then(|j| j.to_str()?.parse::<u32>().ok())
                .or_else(|| Some(std::thread::available_parallelism().ok()?.get() as u32))
                .unwrap_or(4);

            Self(AtomicU32::new(parallelism))
        }

        pub(super) async fn acquire(&self) -> JobToken {
            loop {
                // TODO: use try_fetch once msrv bump to 1.95
                #[allow(deprecated)]
                let res = self
                    .0
                    .fetch_update(AcqRel, Acquire, |tokens| tokens.checked_sub(1));

                if res.is_ok() {
                    break JobToken::new();
                }

                YieldOnce::default().await
            }
        }

        pub(super) fn release_token_raw(&self) {
            self.0.fetch_add(1, AcqRel);
        }
    }
}
