use jobserver::{Acquired, Client, HelperThread};
use std::{
    env,
    mem::MaybeUninit,
    sync::{
        mpsc::{self, Receiver, Sender},
        Once,
    },
};

pub(crate) struct JobToken {
    /// The token can either be a fresh token obtained from the jobserver or - if `token` is None - an implicit token for this process.
    /// Both are valid values to put into queue.
    token: Option<Acquired>,
    /// A pool to which `token` should be returned. `pool` is optional, as one might want to release a token straight away instead
    /// of storing it back in the pool - see [`Self::forget()`] function for that.
    pool: Option<Sender<Option<Acquired>>>,
}

impl Drop for JobToken {
    fn drop(&mut self) {
        if let Some(pool) = &self.pool {
            let _ = pool.send(self.token.take());
        }
    }
}

impl JobToken {
    /// Ensure that this token is not put back into queue once it's dropped.
    /// This also leads to releasing it sooner for other processes to use,
    /// which is a correct thing to do once it is known that there won't be
    /// any more token acquisitions.
    pub(crate) fn forget(&mut self) {
        self.pool.take();
    }
}

/// A thin wrapper around jobserver's Client.
/// It would be perfectly fine to just use jobserver's Client, but we also want to reuse
/// our own implicit token assigned for this build script. This struct manages that and
/// gives out tokens without exposing whether they're implicit tokens or tokens from jobserver.
/// Furthermore, instead of giving up job tokens, it keeps them around
/// for reuse if we know we're going to request another token after freeing the current one.
pub(crate) struct JobTokenServer {
    helper: HelperThread,
    tx: Sender<Option<Acquired>>,
    rx: Receiver<Option<Acquired>>,
}

impl JobTokenServer {
    pub(crate) fn new() -> &'static Self {
        jobserver()
    }
    fn new_inner(client: Client) -> Result<Self, crate::Error> {
        let (tx, rx) = mpsc::channel();
        // Push the implicit token. Since JobTokens only give back what they got,
        // there should be at most one global implicit token in the wild.
        tx.send(None).unwrap();
        let pool = tx.clone();
        let helper = client.into_helper_thread(move |acq| {
            let _ = pool.send(Some(acq.unwrap()));
        })?;
        Ok(Self { helper, tx, rx })
    }

    pub(crate) fn acquire(&self) -> JobToken {
        let token = if let Ok(token) = self.rx.try_recv() {
            // Opportunistically check if there's a token that can be reused.
            token
        } else {
            // Cold path, request a token and block
            self.helper.request_token();
            self.rx.recv().unwrap()
        };
        JobToken {
            token,
            pool: Some(self.tx.clone()),
        }
    }
}

/// Returns a suitable `JobTokenServer` used to coordinate
/// parallelism between build scripts. A global `JobTokenServer` is used as this ensures
/// that only one implicit job token is used in the wild.
/// Having multiple separate job token servers would lead to each of them assuming that they have control
/// over the implicit job token.
/// As it stands, each caller of `jobserver` can receive an implicit job token and there will be at most
/// one implicit job token in the wild.
fn jobserver() -> &'static JobTokenServer {
    static INIT: Once = Once::new();
    static mut JOBSERVER: MaybeUninit<JobTokenServer> = MaybeUninit::uninit();

    fn _assert_sync<T: Sync>() {}
    _assert_sync::<jobserver::Client>();

    unsafe {
        INIT.call_once(|| {
            let server = default_jobserver();
            JOBSERVER = MaybeUninit::new(
                JobTokenServer::new_inner(server).expect("Job server initialization failed"),
            );
        });
        // Poor man's assume_init_ref, as that'd require a MSRV of 1.55.
        &*JOBSERVER.as_ptr()
    }
}

unsafe fn default_jobserver() -> jobserver::Client {
    // Try to use the environmental jobserver which Cargo typically
    // initializes for us...
    if let Some(client) = jobserver::Client::from_env() {
        return client;
    }

    // ... but if that fails for whatever reason select something
    // reasonable and crate a new jobserver. Use `NUM_JOBS` if set (it's
    // configured by Cargo) and otherwise just fall back to a
    // semi-reasonable number. Note that we could use `num_cpus` here
    // but it's an extra dependency that will almost never be used, so
    // it's generally not too worth it.
    let mut parallelism = 4;
    if let Ok(amt) = env::var("NUM_JOBS") {
        if let Ok(amt) = amt.parse() {
            parallelism = amt;
        }
    }

    // If we create our own jobserver then be sure to reserve one token
    // for ourselves.
    let client = jobserver::Client::new(parallelism).expect("failed to create jobserver");
    client.acquire_raw().expect("failed to acquire initial");
    return client;
}
