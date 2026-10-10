#[cfg(unix)]
mod unix;
pub(crate) use unix::*;

#[cfg(not(unix))]
mod unsupported;
pub(crate) use unsupported::*;
