use std::{
    io,
    marker::PhantomData,
    process::ChildStderr,
    thread::sleep,
    time::Duration,
};

#[derive(Default)]
pub(crate) struct Reactor(PhantomData<()>);

pub(crate) struct Registration<'a>(PhantomData<&'a Reactor>);

impl Reactor {
    pub(crate) fn register_stderr(&self, stderr: &ChildStderr) -> io::Result<Registration<'_>> {
        Ok(Registration(PhantomData))
    }

    pub(crate) fn wait_with_timeout(&self, timeout: Duration) -> io::Result<()> {
        sleep(duration);
        Ok(())
    }
}
