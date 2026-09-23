use crate::windows_sys_bindings as win;

// Private API
pub(crate) use win::*;

// Suppress unused warnings for types that can't be excluded from the bindings
#[allow(unused)]
use win::{IID_IUnknown as _, IUnknown_Vtbl as _};

// Public only for cc-rs
pub const FILE_ATTRIBUTE_TEMPORARY: u32 = win::FILE_ATTRIBUTE_TEMPORARY as u32;
pub use win::PeekNamedPipe;
