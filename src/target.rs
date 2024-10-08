//! Very basic parsing of `rustc` target triples.
//!
//! See the `target-lexicon` crate for a more principled approach to this.

use std::{borrow::Cow, env, str::FromStr};

use crate::{Error, ErrorKind};

mod generated;

/// The parts of `rustc`'s target triple.
///
/// See <https://doc.rust-lang.org/cargo/appendix/glossary.html#target>.
#[derive(Debug, PartialEq)]
pub(crate) struct Target {
    /// The full architecture, including the subarchitecture.
    ///
    /// This differs from `cfg!(target_arch)`, which only specifies the
    /// overall architecture, which is too coarse for certain cases.
    pub full_arch: Cow<'static, str>,
    /// The overall target architecture.
    ///
    /// This is the same as the value of `cfg!(target_arch)`.
    pub arch: Cow<'static, str>,
    /// The target vendor.
    ///
    /// This is the same as the value of `cfg!(target_vendor)`.
    pub vendor: Cow<'static, str>,
    /// The operating system, or `none` on bare-metal targets.
    ///
    /// This is the same as the value of `cfg!(target_os)`.
    pub os: Cow<'static, str>,
    /// The environment on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_env)`.
    pub env: Cow<'static, str>,
    /// The ABI on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_abi)`.
    pub abi: Cow<'static, str>,
}

impl Target {
    pub fn from_cargo_environment_variables() -> Result<Self, Error> {
        let getenv = |name| {
            // No need to emit `rerun-if-env-changed` for these variables,
            // as they are controlled by Cargo itself.
            #[allow(clippy::disallowed_methods)]
            env::var(name).map_err(|err| {
                Error::new(
                    ErrorKind::EnvVarNotFound,
                    format!("failed reading {name}: {err}"),
                )
            })
        };

        let target = getenv("TARGET")?;
        let (full_arch, _rest) = target.split_once('-').ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target}` had an unknown architecture"),
        ))?;

        let arch = getenv("CARGO_CFG_TARGET_ARCH")?;
        let vendor = getenv("CARGO_CFG_TARGET_VENDOR")?;
        let os = getenv("CARGO_CFG_TARGET_OS")?;
        let env = getenv("CARGO_CFG_TARGET_ENV")?;
        // `target_abi` was stabilized in Rust 1.78, so may not always be available.
        let abi = if let Ok(abi) = getenv("CARGO_CFG_TARGET_ABI") {
            abi.into()
        } else {
            Self::from_str(&target)?.abi
        };

        Ok(Self {
            full_arch: full_arch.to_string().into(),
            arch: arch.into(),
            vendor: vendor.into(),
            os: os.into(),
            env: env.into(),
            abi,
        })
    }
}

impl FromStr for Target {
    type Err = Error;

    fn from_str(target_triple: &str) -> Result<Self, Error> {
        if let Some(target) = generated::get(target_triple) {
            Ok(target)
        } else {
            Err(Error::new(
                ErrorKind::InvalidTarget,
                format!("unknown target `{target_triple}`"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Target;

    // Test tier 1 targets
    #[test]
    fn tier1() {
        let targets = [
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "i686-pc-windows-gnu",
            "i686-pc-windows-msvc",
            "i686-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-gnu",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
        ];

        for target in targets {
            // Check that it parses
            let _ = Target::from_str(target).unwrap();
        }
    }

    // Various custom target triples not (or no longer) known by `rustc`
    #[test]
    fn cannot_parse_extra() {
        let targets = [
            "arm-frc-linux-gnueabi",
            "aarch64-uwp-windows-gnu",
            "arm-unknown-netbsd-eabi",
            "armv7neon-unknown-linux-gnueabihf",
            "armv7neon-unknown-linux-musleabihf",
            "thumbv7-unknown-linux-gnueabihf",
            "thumbv7-unknown-linux-musleabihf",
            "x86_64-rumprun-netbsd",
        ];

        for target in targets {
            // Check that it does not parse
            let _ = Target::from_str(target).unwrap_err();
        }
    }
}
