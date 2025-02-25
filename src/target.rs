//! Very basic parsing of `rustc` target triples.
//!
//! See the `target-lexicon` crate for a more principled approach to this.

mod apple;
mod llvm;
mod parser;

pub(crate) use parser::TargetInfoParser;

/// Information specific to a `rustc` target.
///
/// See <https://doc.rust-lang.org/cargo/appendix/glossary.html#target>.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TargetInfo<'a> {
    /// The full architecture, including the subarchitecture.
    ///
    /// This differs from `cfg!(target_arch)`, which only specifies the
    /// overall architecture, which is too coarse for certain cases.
    pub full_arch: &'a str,
    /// The overall target architecture.
    ///
    /// This is the same as the value of `cfg!(target_arch)`.
    pub arch: &'a str,
    /// The target vendor.
    ///
    /// This is the same as the value of `cfg!(target_vendor)`.
    pub vendor: &'a str,
    /// The operating system, or `none` on bare-metal targets.
    ///
    /// This is the same as the value of `cfg!(target_os)`.
    pub os: &'a str,
    /// The environment on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_env)`.
    pub env: &'a str,
    /// The ABI on top of the operating system.
    ///
    /// This is the same as the value of `cfg!(target_abi)`.
    pub abi: &'a str,
}

#[cfg(test)]
mod tests {
    use std::process::Command;

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
            let _ = TargetInfo::from_rustc_target(target).unwrap();
        }
    }

    fn target_from_rustc_cfgs<'a>(target: &'a str, cfgs: &'a str) -> TargetInfo<'a> {
        // Cannot determine full architecture from cfgs.
        let (full_arch, _rest) = target.split_once('-').expect("target to have arch");

        let mut target = TargetInfo {
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
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

        let target_list = Command::new(&rustc)
            .arg("--print=target-list")
            .output()
            .unwrap()
            .stdout;
        let target_list = String::from_utf8(target_list).unwrap();

        let mut has_failure = false;
        for target in target_list.lines() {
            let cfgs = Command::new(&rustc)
                .arg("--target")
                .arg(target)
                .arg("--print=cfg")
                .output()
                .unwrap()
                .stdout;
            let cfgs = String::from_utf8(cfgs).unwrap();

            let expected = target_from_rustc_cfgs(target, &cfgs);
            let actual = TargetInfo::from_rustc_target(target);

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
            "aarch64-unknown-none-gnu",
            "aarch64-uwp-windows-gnu",
            "arm-frc-linux-gnueabi",
            "arm-unknown-netbsd-eabi",
            "armv7neon-unknown-linux-gnueabihf",
            "armv7neon-unknown-linux-musleabihf",
            "thumbv7-unknown-linux-gnueabihf",
            "thumbv7-unknown-linux-musleabihf",
            "armv7-apple-ios",
            "wasm32-wasi",
            "x86_64-rumprun-netbsd",
            "x86_64-unknown-linux",
            "x86_64-alpine-linux-musl",
            "x86_64-chimera-linux-musl",
            "x86_64-foxkit-linux-musl",
            "arm-poky-linux-gnueabi",
            "x86_64-unknown-moturus",
        ];

        for target in targets {
            // Check that it parses
            let _ = TargetInfo::from_rustc_target(target).unwrap();
        }
    }
}
