#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(not(unix))]
mod unsupported;
#[cfg(not(unix))]
pub(crate) use unsupported::*;
