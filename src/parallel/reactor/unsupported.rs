use std::{
  marker::PhantomData,
  process::ChildStderr,
  thread::sleep,
  time::Duration,
};

#[derive(Default)]
pub(crate) struct Reactor(PhantomData<()>);

pub(crate) struct Registration<'a>(PhantomData<&'a Reactor>);

impl Reactor {
    pub(crate) fn register_stderr(&self, stderr: &ChildStderr) -> Registration<'_> {
        Registration(PhantomData)
    }

    pub(crate) fn wait_with_timeout(&self, timeout: Duration) {
      sleep(duration);
    }
}
