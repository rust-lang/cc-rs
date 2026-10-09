use std::{
    cell::UnsafeCell,
    ffi::{OsStr, OsString},
    fmt::{self, Write},
    hash::Hasher,
    marker::PhantomData,
    mem::MaybeUninit,
    panic::{RefUnwindSafe, UnwindSafe},
    path::Path,
    sync::Once,
};

use crate::{Error, ErrorKind};

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

pub(crate) struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<MaybeUninit<T>>,
    _marker: PhantomData<T>,
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OnceLock<T> {
    pub(crate) const fn new() -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _marker: PhantomData,
        }
    }

    #[inline]
    fn is_initialized(&self) -> bool {
        self.once.is_completed()
    }

    unsafe fn get_unchecked(&self) -> &T {
        debug_assert!(self.is_initialized());
        #[allow(clippy::needless_borrow)]
        #[allow(unused_unsafe)]
        unsafe {
            (&*self.value.get()).assume_init_ref()
        }
    }

    pub(crate) fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.once.call_once(|| {
            unsafe { &mut *self.value.get() }.write(f());
        });
        unsafe { self.get_unchecked() }
    }

    pub(crate) fn get(&self) -> Option<&T> {
        self.is_initialized().then(|| {
            // SAFETY: `is_initialized()` returned `true`, so the value is initialized.
            unsafe { self.get_unchecked() }
        })
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(value: T) -> Self {
        let cell = Self::new();
        cell.get_or_init(|| value);
        cell
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> Self {
        let cell = Self::new();
        if let Some(value) = self.get() {
            cell.get_or_init(|| value.clone());
        }
        cell
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for OnceLock<T> {}
impl<T: UnwindSafe> UnwindSafe for OnceLock<T> {}

impl<T> Drop for OnceLock<T> {
    #[inline]
    fn drop(&mut self) {
        if self.once.is_completed() {
            // SAFETY: The cell is initialized and being dropped, so it can't
            // be accessed again.
            unsafe { self.value.get_mut().assume_init_drop() };
        }
    }
}

/// Access an environment variable that's set by Cargo.
///
/// <https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts>
///
/// Cargo doesn't need to be told about these with `rerun-if-env-changed`, and
/// that we don't want to allow overwriting them with `Build::env`.
#[allow(clippy::disallowed_methods)] // Cargo env, no need for cache busting.
pub(crate) fn cargo_env_var_os(key: &str) -> Option<OsString> {
    std::env::var_os(key)
}

pub(crate) fn cargo_env_var(key: &str) -> Result<String, Error> {
    if let Some(value) = cargo_env_var_os(key) {
        match value.into_string() {
            Ok(value) => Ok(value),
            Err(value) => Err(Error::new(
                ErrorKind::EnvVarNotFound,
                format!("environment variable {key} is not valid utf-8: {value:?}"),
            )),
        }
    } else {
        Err(Error::new(
            ErrorKind::EnvVarNotFound,
            format!("environment variable {key} not defined"),
        ))
    }
}

/// `contains`, `starts_with` and `ends_with` that ignore ASCII case, like
/// `str::eq_ignore_ascii_case`. Check program names with these (or with
/// `eq_ignore_ascii_case`), since file names are case insensitive on Windows.
pub(crate) trait IgnoreAsciiCase {
    fn contains_ignore_ascii_case(&self, needle: &str) -> bool;
    fn starts_with_ignore_ascii_case(&self, prefix: &str) -> bool;
    fn ends_with_ignore_ascii_case(&self, suffix: &str) -> bool;
}

impl IgnoreAsciiCase for str {
    fn contains_ignore_ascii_case(&self, needle: &str) -> bool {
        needle.is_empty()
            || self
                .as_bytes()
                .windows(needle.len())
                .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    }

    fn starts_with_ignore_ascii_case(&self, prefix: &str) -> bool {
        self.as_bytes()
            .get(..prefix.len())
            .map_or(false, |start| start.eq_ignore_ascii_case(prefix.as_bytes()))
    }

    fn ends_with_ignore_ascii_case(&self, suffix: &str) -> bool {
        self.len().checked_sub(suffix.len()).map_or(false, |start| {
            self.as_bytes()[start..].eq_ignore_ascii_case(suffix.as_bytes())
        })
    }
}

/// A [`Hasher`] that keeps what is written to it, to turn a value into bytes
/// that are equal exactly when the values are, as long as its `Hash` impl is
/// prefix-free like those of `OsStr`, `str` and slices.
#[derive(Default)]
pub(crate) struct HashRecorder(Vec<u8>);

impl HashRecorder {
    pub(crate) fn into_bytes(self) -> Box<[u8]> {
        self.0.into_boxed_slice()
    }
}

impl Hasher for HashRecorder {
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        unreachable!("only records what is written")
    }
}

#[cfg(test)]
mod tests {
    use super::IgnoreAsciiCase;

    #[test]
    fn ignore_ascii_case() {
        assert!("x86_64-w64-mingw32-CLANG".contains_ignore_ascii_case("mingw32-clang"));
        assert!("Clang-CL.exe".contains_ignore_ascii_case("clang-cl"));
        assert!("zig".contains_ignore_ascii_case(""));
        assert!(!"zi".contains_ignore_ascii_case("zig"));
        assert!(!"clang".contains_ignore_ascii_case("clang-cl"));

        assert!("LLVM-ML64".starts_with_ignore_ascii_case("llvm-ml"));
        assert!(!"llvm-m".starts_with_ignore_ascii_case("llvm-ml"));
        assert!(!"my-llvm-ml".starts_with_ignore_ascii_case("llvm-ml"));

        assert!("x86_64-w64-mingw32-Clang++".ends_with_ignore_ascii_case("-mingw32-clang++"));
        assert!("CL".ends_with_ignore_ascii_case("cl"));
        assert!(!"l".ends_with_ignore_ascii_case("cl"));
        assert!(!"cl.exe".ends_with_ignore_ascii_case("cl"));

        // Only ASCII letters are folded, and other characters still compare.
        assert!("Ünïcode-CLANG".contains_ignore_ascii_case("Ünïcode-clang"));
        assert!(!"ÜNÏCODE-clang".contains_ignore_ascii_case("ünïcode-clang"));
        assert!(!"\u{212A}ache".starts_with_ignore_ascii_case("kache"));
    }
}
