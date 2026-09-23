use crate::windows_sys_bindings as win;

// Private API
pub(crate) use win::*;

// Public only for cc-rs
pub const FILE_ATTRIBUTE_TEMPORARY: u32 = win::FILE_ATTRIBUTE_TEMPORARY as u32;
pub use win::PeekNamedPipe;
