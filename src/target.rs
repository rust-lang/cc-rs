//! Very basic parsing of `rustc` target triples.
//!
//! See the `target-lexicon` crate for a more principled approach to this.

use std::{env, str::FromStr};

use crate::{Error, ErrorKind};

/// The parts of `rustc`'s target triple.
///
/// See <https://doc.rust-lang.org/cargo/appendix/glossary.html#target>.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Target {
    /// The full architecture, including the subarchitecture.
    ///
    /// This differs from `cfg!(target_arch)`, which only specifies the
    /// overall architecture, which is too coarse for certain cases.
    pub full_arch: String,
    /// The overall target architecture.
    ///
    /// This is the same as the value of `cfg!(target_arch)`.
    pub arch: String,
    /// The target vendor.
    ///
    /// This is the same as the value of `cfg!(target_vendor)`.
    pub vendor: String,
    /// The operating system, or `none` on bare-metal targets.
    ///
    /// This is the same as the value of `cfg!(target_os)`.
    pub os: String,
    /// The environment on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_env)`.
    pub env: String,
    /// The ABI on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_abi)`.
    pub abi: String,
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
            abi
        } else {
            Self::from_str(&target)?.abi
        };

        Ok(Self {
            full_arch: full_arch.to_string(),
            arch,
            vendor,
            os,
            env,
            abi,
        })
    }
}

impl FromStr for Target {
    type Err = Error;

    fn from_str(target: &str) -> Result<Self, Error> {
        let mut components = target.split('-');

        // Insist that the triple start with a valid architecture.
        let full_arch = components.next().ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target}` was empty"),
        ))?;

        let arch = match full_arch {
            // FIXME(rustc): This is probably the wrong arch, should be `aarch64`?
            "arm64ec" => "arm64ec",
            "asmjs" => "wasm32",
            "avr" => "avr",
            "bpfeb" | "bpfel" => "bpf",
            "csky" => "csky",
            "hexagon" => "hexagon",
            "i386" | "i586" | "i686" => "x86",
            "loongarch64" => "loongarch64",
            "m68k" => "m68k",
            "mipsisa32r6" | "mipsisa32r6el" => "mips32r6",
            "mipsisa64r6" | "mipsisa64r6el" => "mips64r6",
            "mips64" | "mips64el" => "mips64",
            "mips" | "mipsel" => "mips",
            "msp430" => "msp430",
            "nvptx64" => "nvptx64",
            "powerpc" => "powerpc",
            "powerpc64" | "powerpc64le" => "powerpc64",
            "s390x" => "s390x",
            "sparc" => "sparc",
            "sparc64" | "sparcv9" => "sparc64",
            "wasm32" => "wasm32",
            "wasm64" => "wasm64",
            "x86_64" | "x86_64h" => "x86_64",
            "xtensa" => "xtensa",
            aarch64 if aarch64.starts_with("aarch64") | aarch64.starts_with("arm64") => "aarch64",
            arm if arm.starts_with("arm") | arm.starts_with("thumb") => "arm",
            riscv32 if riscv32.starts_with("riscv32") => "riscv32",
            riscv64 if riscv64.starts_with("riscv64") => "riscv64",
            _ => {
                // TODO: Should we warn on unknown architectures instead?
                return Err(Error::new(
                    ErrorKind::ArchitectureInvalid,
                    format!("target `{target}` had an unknown architecture"),
                ));
            }
        };

        let maybe_vendor = components.next().ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target}` must have at least two components"),
        ))?;

        // Lately, newer target triples have begun omitting the vendor. To
        // still be able to parse these, we match it against a list of known
        // vendors here, and assume that if it doesn't match, that the triple
        // does not contain a vendor.
        let mut has_vendor = true;
        let mut vendor = match maybe_vendor {
            "unknown" => "unknown",
            "amd" => "amd",
            "apple" => "apple",
            // esp, esp32, esp32s2 etc.
            esp if esp.starts_with("esp") => "espressif",
            "fortanix" => "fortanix",
            "frc" => "frc",
            "ibm" => "ibm",
            "kmc" => "kmc",
            "nintendo" => "nintendo",
            "nvidia" => "nvidia",
            // FIXME(rustc): Seems unintentional? OpenWRT is the OS?
            "openwrt" => "unknown",
            "pc" => "pc",
            "risc0" => "risc0",
            "rumprun" => "rumprun",
            "sony" => "sony",
            "sun" => "sun",
            "unikraft" => "unikraft",
            "uwp" => "uwp",
            // FIXME(rustc): win7 does not really make sense as a vendor?
            "win7" => "win7",
            "wrs" => "wrs",
            _ => {
                has_vendor = false;
                "unknown"
            }
        };

        let os = if has_vendor {
            components.next().ok_or(Error::new(
                ErrorKind::InvalidTarget,
                format!("target `{target}` must have at least two components"),
            ))?
        } else {
            // The triple does not contain a vendor, so that part is the os.
            maybe_vendor
        };

        let env_and_abi = components.next().unwrap_or("");

        if components.next().is_some() {
            return Err(Error::new(
                ErrorKind::InvalidTarget,
                format!("too many components in target `{target}`"),
            ));
        }

        // Parse actual OS out of the two last components.
        let (mut os, env_and_abi) = match (os, env_and_abi) {
            (wasi, _) if wasi.starts_with("wasi") => {
                let mut env = wasi.strip_prefix("wasi").unwrap();
                if env.is_empty() {
                    // Currently transitioning some targets here, remove once transition is over:
                    // <https://blog.rust-lang.org/2024/04/09/updates-to-rusts-wasi-targets.html>
                    env = "p1";
                }
                ("wasi", env)
            }
            // Android is the actual OS name, `linux` in the target name is legacy.
            ("linux", "android") => ("android", ""),
            ("linux", "androideabi") => ("android", "eabi"),
            // Horizon is the common OS name between the 3DS and the Switch.
            ("3ds", "") => ("horizon", "newlibeabihf"),
            ("switch", "freestanding") => {
                // FIXME(rustc): Is the missing vendor intentional?
                vendor = "unknown";
                ("horizon", "")
            }
            // FIXME(rustc): `mipsel-sony-psx` has no OS component.
            ("psx", "") => ("none", "psx"),
            // macOS targets are badly named.
            ("darwin", env_and_abi) => ("macos", env_and_abi),
            (os, env_and_abi) => (os, env_and_abi),
        };

        // Parse environment and ABI.
        let (mut env, mut abi) = match (os, env_and_abi) {
            ("windows", env) if vendor == "uwp" => (env, "uwp"),
            ("vxworks", abi) => ("gnu", abi),
            ("rtems", abi) => ("newlib", abi),
            ("espidf", abi) => ("newlib", abi),
            ("redox", abi) => ("relibc", abi),
            ("aix", env) => (env, "vec-extabi"),
            ("unknown", "sgx") => ("sgx", "fortanix"),
            (_, "sim") => ("", "sim"),
            (_, "macabi") => ("", "macabi"),
            (_, gnu) if gnu.starts_with("gnu") => {
                let abi = gnu.strip_prefix("gnu").unwrap();
                // To handle gnu_ilp32
                let abi = abi.strip_prefix("_").unwrap_or(abi);
                ("gnu", abi)
            }
            (_, musl) if musl.starts_with("musl") => {
                let abi = musl.strip_prefix("musl").unwrap();
                ("musl", abi)
            }
            (_, "softfloat") => ("", "softfloat"),
            (_, "qnx700") => ("nto70", ""),
            (_, "qnx710") => ("nto71", ""),
            (_, "elf") => ("", ""),
            (_, "none") => ("", ""),
            (_, "eabi") => ("", "eabi"),
            (_, "eabihf") => ("", "eabihf"),
            (_, "uclibceabi") => ("uclibc", "eabi"),
            (_, "uclibceabihf") => ("uclibc", "eabihf"),
            (_, "newlibeabihf") => ("newlib", "eabihf"),
            // General fallback
            (_, env) => (env, ""),
        };

        // FIXME(rustc): The name has `pc` as the vendor, but it's not set?
        if os == "nto" {
            vendor = "unknown";
        }

        // FIXME(rustc): Vendor isn't set for these targets?
        if vendor == "espressif" && os == "none" {
            vendor = "unknown";
        }

        // Extra overrides for badly named targets.
        match target {
            // Actually simulator targets.
            "i386-apple-ios" | "x86_64-apple-ios" | "x86_64-apple-tvos" => {
                abi = "sim";
            }
            // Name should've contained `muslabi64`.
            "mips64-openwrt-linux-musl" => {
                abi = "abi64";
            }
            // Has no OS name
            "avr-unknown-gnu-atmega328" => {
                os = "none";
                // FIXME(rustc): Should this specify `gnu` env?
                env = "";
                abi = "";
            }
            // Specifies abi even though not in name
            "armv6-unknown-freebsd" | "armv7-unknown-freebsd" => {
                // FIXME(rustc): Is it a bug that the env is set to "gnu" here?
                env = "gnu";
                abi = "eabihf";
            }
            // Specifies abi even though not in name
            "armv7-unknown-linux-ohos" | "armv7-unknown-trusty" => {
                abi = "eabi";
            }
            // FIXME(rustc): Is it a bug that the ABI isn't set here?
            "armv7a-kmc-solid_asp3-eabi" | "armv7a-kmc-solid_asp3-eabihf" => {
                abi = "";
            }
            // FIXME(rustc): Specifies "elf" ABI, is that correct?
            "sparc-unknown-none-elf" => {
                abi = "elf";
            }
            _ => {}
        }

        Ok(Target {
            full_arch: full_arch.into(),
            arch: arch.into(),
            vendor: vendor.into(),
            os: os.into(),
            env: env.into(),
            abi: abi.into(),
        })
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
            full_arch: full_arch.into(),
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

                let value = value.to_string();
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
    fn parse_extra() {
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
            // Check that it parses
            let _ = Target::from_str(target).unwrap();
        }
    }
}
