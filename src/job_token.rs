use jobserver::{Acquired, Client, HelperThread};
use std::sync::mpsc::{self, Receiver, Sender};

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
    /// This also leads to releasing it sooner for other processes to use, which is a good thing to do once you know that
    /// you're never going to request a token in this process again.
    pub(crate) fn forget(&mut self) {
        self.should_return_to_queue = false;
    }
}

/// A thin wrapper around jobserver's Client.
/// It would be perfectly fine to just use that, but we also want to reuse our own implicit token assigned for this build script.
/// This struct manages that and gives out tokens without exposing whether they're implicit tokens or tokens from jobserver.
/// Furthermore, instead of giving up job tokens, it keeps them around for reuse if we know we're going to request another token after freeing the current one.
pub(crate) struct JobTokenServer {
    helper: HelperThread,
    tx: Sender<Option<Acquired>>,
    rx: Receiver<Option<Acquired>>,
}

impl JobTokenServer {
    pub(crate) fn new(client: Client) -> Result<Self, crate::Error> {
        let (tx, rx) = mpsc::channel();
        // Initialize the
        tx.send(None).unwrap();
        let pool = tx.clone();
        let helper = client.into_helper_thread(move |acq| {
            let _ = pool.send(Some(acq.unwrap()));
        })?;
        Ok(Self { helper, tx, rx })
    }

    pub(crate) fn acquire(&mut self) -> JobToken {
        let token = if let Ok(token) = self.rx.try_recv() {
            // Opportunistically check if we already have a token for our own reuse.
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
