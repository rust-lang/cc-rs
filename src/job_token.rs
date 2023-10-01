use jobserver::{Acquired, Client, HelperThread};
use std::{
    env,
    sync::{
        mpsc::{self, Receiver, Sender},
        Once,
    },
};

pub(crate) struct JobToken {
    /// The token can either be a fresh token obtained from the jobserver or - if `token` is None - an implicit token for this process.
    /// Both are valid values to put into queue.
    token: Option<Acquired>,
    pool: Sender<Option<Acquired>>,
    should_return_to_queue: bool,
}

impl Drop for JobToken {
    fn drop(&mut self) {
        if self.should_return_to_queue {
            let _ = self.pool.send(self.token.take());
        }
    }
}

impl JobToken {
    /// Ensure that this token is not put back into queue once it's dropped.
    /// This also leads to releasing it sooner for other processes to use,
    /// which is a correct thing to do once it is known that there won't be
    /// any more token acquisitions.
    pub(crate) fn forget(&mut self) {
        self.should_return_to_queue = false;
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
    pub(crate) fn new(client: Client) -> Result<Self, crate::Error> {
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

    pub(crate) fn acquire(&mut self) -> JobToken {
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
            pool: self.tx.clone(),
            should_return_to_queue: true,
        }
    }
}

/// Returns a suitable `jobserver::Client` used to coordinate
/// parallelism between build scripts.
pub(super) fn jobserver() -> jobserver::Client {
    static INIT: Once = Once::new();
    static mut JOBSERVER: Option<jobserver::Client> = None;

    fn _assert_sync<T: Sync>() {}
    _assert_sync::<jobserver::Client>();

    unsafe {
        INIT.call_once(|| {
            let server = default_jobserver();
            JOBSERVER = Some(server);
        });
        JOBSERVER.clone().unwrap()
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
