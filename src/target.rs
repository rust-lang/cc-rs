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
    use std::{process::Command, str::FromStr};

    use super::Target;

    fn target_from_cfgs(target: &str, cfgs: &str) -> Target {
        // Cannot determine full architecture from cfgs.
        let (full_arch, _rest) = target.split_once('-').expect("target to have arch");

        let mut target = Target {
            full_arch: full_arch.to_string().into(),
            arch: "invalid-none-set".into(),
            vendor: "invalid-none-set".into(),
            os: "invalid-none-set".into(),
            env: "invalid-none-set".into(),
            // Not set in older Rust versions
            abi: "".into(),
        };

        for cfg in cfgs.lines() {
            if let Some((name, value)) = cfg.split_once('=') {
                // Remove whitespace, if `rustc` decided to insert any
                let name = name.trim();
                let value = value.trim();

                // Remove quotes around value
                let value = value.strip_prefix('"').unwrap_or(value);
                let value = value.strip_suffix('"').unwrap_or(value);

                let value = value.to_string().into();
                match name {
                    "target_arch" => target.arch = value,
                    "target_vendor" => target.vendor = value,
                    "target_os" => target.os = value,
                    "target_env" => target.env = value,
                    "target_abi" => target.abi = value,
                    _ => {}
                }
            } else {
                // Skip cfgs like `debug_assertions` and `unix`.
            }
        }

        target
    }

    #[test]
    fn parse_rustc_targets() {
        let target_list = Command::new("rustc")
            .arg("--print=target-list")
            .output()
            .unwrap()
            .stdout;
        let target_list = String::from_utf8(target_list).unwrap();

        let mut has_failure = false;
        for target in target_list.lines() {
            let cfgs = Command::new("rustc")
                .arg("--target")
                .arg(target)
                .arg("--print=cfg")
                .output()
                .unwrap()
                .stdout;
            let cfgs = String::from_utf8(cfgs).unwrap();

            let expected = target_from_cfgs(target, &cfgs);
            let actual = Target::from_str(target);

            if Some(&expected) != actual.as_ref().ok() {
                eprintln!("failed comparing {target}:");
                eprintln!("  expected: Ok({expected:?})");
                eprintln!("    actual: {actual:?}");
                eprintln!();
                has_failure = true;
            }
        }

        if has_failure {
            panic!("failed comparing targets");
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
