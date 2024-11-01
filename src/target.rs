//! Very basic parsing of `rustc` target triples.
//!
//! See the `target-lexicon` crate for a more principled approach to this.

use std::{borrow::Cow, env, str::FromStr};

use crate::{Error, ErrorKind};

mod apple;
mod generated;
mod llvm;

/// Information specific to a `rustc` target.
///
/// See <https://doc.rust-lang.org/cargo/appendix/glossary.html#target>.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TargetInfo {
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
    /// The unversioned LLVM/Clang target triple.
    unversioned_llvm_target: Cow<'static, str>,
}

impl TargetInfo {
    pub fn from_cargo_environment_variables() -> Result<Self, Error> {
        // `TARGET` must be present.
        //
        // No need to emit `rerun-if-env-changed` for this,
        // as it is controlled by Cargo itself.
        #[allow(clippy::disallowed_methods)]
        let target_triple = env::var("TARGET").map_err(|err| {
            Error::new(
                ErrorKind::EnvVarNotFound,
                format!("failed reading TARGET: {err}"),
            )
        })?;

        // Parse the full architecture name from the target triple.
        let (full_arch, _rest) = target_triple.split_once('-').ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target_triple}` had an unknown architecture"),
        ))?;

        let cargo_env = |name, fallback| {
            // No need to emit `rerun-if-env-changed` for these,
            // as they are controlled by Cargo itself.
            #[allow(clippy::disallowed_methods)]
            match env::var(name) {
                Ok(var) => Ok(Cow::Owned(var)),
                Err(err) => match fallback {
                    Some(fallback) => Ok(fallback),
                    None => Err(Error::new(
                        ErrorKind::EnvVarNotFound,
                        format!("did not find fallback information for target `{target_triple}`, and failed reading {name}: {err}"),
                    )),
                },
            }
        };

        // Prefer to use `CARGO_ENV_*` if set, since these contain the most
        // correct information relative to the current `rustc`, and makes it
        // possible to support custom target JSON specs unknown to `rustc`.
        //
        // NOTE: If the user is using an older `rustc`, that data may be older
        // than our pre-generated data, but we still prefer Cargo's view of
        // the world, since at least `cc` won't differ from `rustc` in that
        // case.
        //
        // These may not be set in case the user depended on being able to
        // just set `TARGET` outside of build scripts; in those cases, fall
        // back back to data from the known set of target triples instead.
        //
        // See discussion in #1225 for further details.
        let fallback_target = TargetInfo::from_str(&target_triple).ok();
        let ft = fallback_target.as_ref();
        let arch = cargo_env("CARGO_CFG_TARGET_ARCH", ft.map(|t| t.arch.clone()))?;
        let vendor = cargo_env("CARGO_CFG_TARGET_VENDOR", ft.map(|t| t.vendor.clone()))?;
        let os = cargo_env("CARGO_CFG_TARGET_OS", ft.map(|t| t.os.clone()))?;
        let env = cargo_env("CARGO_CFG_TARGET_ENV", ft.map(|t| t.env.clone()))?;
        // `target_abi` was stabilized in Rust 1.78, which is higher than our
        // MSRV, so it may not always be available; In that case, fall back to
        // `""`, which is _probably_ correct for unknown target triples.
        let abi = cargo_env("CARGO_CFG_TARGET_ABI", ft.map(|t| t.abi.clone()))
            .unwrap_or(Cow::Borrowed(""));

        // Prefer `rustc`'s LLVM target triple information.
        let unversioned_llvm_target = match fallback_target {
            Some(ft) => ft.unversioned_llvm_target,
            None => llvm::guess_llvm_target_triple(full_arch, &vendor, &os, &env, &abi).into(),
        };

        Ok(Self {
            full_arch: full_arch.to_string().into(),
            arch,
            vendor,
            os,
            env,
            abi,
            unversioned_llvm_target,
        })
    }
}

impl FromStr for TargetInfo {
    type Err = Error;

    /// This will fail when using a custom target triple unknown to `rustc`.
    fn from_str(target_triple: &str) -> Result<Self, Error> {
        if let Ok(index) =
            generated::LIST.binary_search_by_key(&target_triple, |(target_triple, _)| target_triple)
        {
            let (_, info) = &generated::LIST[index];
            Ok(info.clone())
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

    use super::TargetInfo;

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
            let _ = TargetInfo::from_str(target).unwrap();
        }
    }

    // Various custom target triples not (or no longer) known by `rustc`
    #[test]
    fn cannot_parse_extra() {
        let targets = [
            "aarch64-unknown-none-gnu",
            "aarch64-uwp-windows-gnu",
            "arm-frc-linux-gnueabi",
            "arm-unknown-netbsd-eabi",
            "armv7neon-unknown-linux-gnueabihf",
            "armv7neon-unknown-linux-musleabihf",
            "thumbv7-unknown-linux-gnueabihf",
            "thumbv7-unknown-linux-musleabihf",
            "x86_64-rumprun-netbsd",
            "x86_64-unknown-linux",
        ];

        for target in targets {
            // Check that it does not parse
            let _ = TargetInfo::from_str(target).unwrap_err();
        }
    }
}
