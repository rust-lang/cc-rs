use std::{
    env,
    ffi::{OsStr, OsString},
    sync::{Mutex, MutexGuard},
    thread::ThreadId,
};

/// A lock for environment variables in the current process.
///
/// A lot of tests need to modify the global environment. This struct ensures
/// that such accesses are serialized, and reverted once the `GlobalEnv` is
/// done being used (to avoid influencing other tests).
///
/// This _does_ make running the tests slower (as they have to run serially),
/// there's two ways to improve that:
/// 1. Use `cargo nextest run`
/// 2. Split integration tests into multiple files - that makes Cargo run them
///    in multiple processes, though with the tradeoff that they're compiled
///    and linked separately (so it might not be faster).
pub struct GlobalEnv {
    lock: MutexGuard<'static, ()>,
    /// The `(key, value)` pairs that we overwrote, in order.
    overwritten_envs: Vec<(OsString, Option<OsString>)>,
}

/// Ignore poisoning, that just means that another test failed, and we don't
/// want that to fail this test.
fn lock_ignore_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(lock) => lock,
        Err(poison_err) => poison_err.into_inner(),
    }
}

static THREAD_CURRENTLY_ACCESSING_ENV: Mutex<Option<ThreadId>> = Mutex::new(None);

impl GlobalEnv {
    #[track_caller]
    pub fn lock() -> Self {
        static GLOBAL_ENV: Mutex<()> = Mutex::new(());

        // Give a better error when calling `GlobalEnv::lock()` on the same
        // thread (which would deadlock).
        if let Some(thread_id) = *lock_ignore_poison(&THREAD_CURRENTLY_ACCESSING_ENV) {
            assert_ne!(
                thread_id,
                std::thread::current().id(),
                "called `GlobalEnv::lock()` on the same thread twice"
            );
        }

        // Acquire the global env lock. Probably waits for a long time while other tests execute.
        let lock = lock_ignore_poison(&GLOBAL_ENV);

        *lock_ignore_poison(&THREAD_CURRENTLY_ACCESSING_ENV) = Some(std::thread::current().id());

        Self {
            lock,
            overwritten_envs: vec![],
        }
    }

    #[allow(clippy::disallowed_methods)]
    pub fn set(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        let key = key.as_ref().into();
        let previous_value = env::var_os(&key);

        // SAFETY: We've taken the `GLOBAL_ENV` lock, so we should be the only
        // thread accessing the environment right now.
        unsafe { env::set_var(&key, value) };

        self.overwritten_envs.push((key, previous_value));
    }

    #[allow(clippy::disallowed_methods)]
    pub fn remove(&mut self, key: impl AsRef<OsStr>) {
        let key = key.as_ref().into();
        let previous_value = env::var_os(&key);

        // SAFETY: We've taken the `GLOBAL_ENV` lock, so we should be the only
        // thread accessing the environment right now.
        unsafe { env::remove_var(&key) };

        self.overwritten_envs.push((key, previous_value));
    }
}

impl Drop for GlobalEnv {
    #[allow(clippy::disallowed_methods)]
    fn drop(&mut self) {
        // "rollback" the changes we made to the environment.

        for (key, previous_value) in self.overwritten_envs.iter().rev() {
            if let Some(value) = previous_value {
                // SAFETY: The GLOBAL_ENV lock is still held here.
                unsafe { env::set_var(key, value) };
            } else {
                // SAFETY: The GLOBAL_ENV lock is still held here.
                unsafe { env::remove_var(key) };
            }
        }

        let mut thread_lock = lock_ignore_poison(&THREAD_CURRENTLY_ACCESSING_ENV);
        *thread_lock = None;
        drop(thread_lock);
    }
}
