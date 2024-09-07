use std::{
    ffi::OsStr,
    fmt::{self, Write},
    path::Path,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub(super) struct JoinOsStrs<'a, T> {
    pub(super) slice: &'a [T],
    pub(super) delimiter: char,
}

impl<T> fmt::Display for JoinOsStrs<'_, T>
where
    T: AsRef<OsStr>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.slice.len();
        for (index, os_str) in self.slice.iter().enumerate() {
            // TODO: Use OsStr::display once it is stablised,
            // Path and OsStr has the same `Display` impl
            write!(f, "{}", Path::new(os_str).display())?;
            if index + 1 < len {
                f.write_char(self.delimiter)?;
            }
        }
        Ok(())
    }
}

pub(super) struct OptionOsStrDisplay<T>(pub(super) Option<T>);

impl<T> fmt::Display for OptionOsStrDisplay<T>
where
    T: AsRef<OsStr>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Use OsStr::display once it is stablised
        // Path and OsStr has the same `Display` impl
        if let Some(os_str) = self.0.as_ref() {
            write!(f, "Some({})", Path::new(os_str).display())
        } else {
            f.write_str("None")
        }
    }
}

#[derive(Default)]
pub(crate) struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        let flag = &mut std::pin::Pin::into_inner(self).0;
        if !*flag {
            *flag = true;
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
