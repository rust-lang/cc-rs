use super::TargetInfo;

impl TargetInfo<'_> {
    /// The LLVM/Clang target triple.
    ///
    /// Rust and Clang don't really agree on target naming, so instead of
    /// inferring this from `TARGET`, we construct it from scratch. See also
    /// <https://clang.llvm.org/docs/CrossCompilation.html#target-triple>.
    ///
    /// NOTE: You should never need to match on this explicitly, use the
    /// fields on [`TargetInfo`] instead.
    pub(crate) fn llvm_target(&self, version: Option<&str>) -> String {
        let arch = match self.full_arch {
            riscv32 if riscv32.starts_with("riscv32") => "riscv32",
            riscv64 if riscv64.starts_with("riscv64") => "riscv64",
            "aarch64" if self.vendor == "apple" => "arm64",
            "armv7" if self.vendor == "sony" => "thumbv7a", // FIXME
            arch => arch,
        };
        let vendor = match self.vendor {
            "kmc" | "nintendo" => "unknown",
            "unknown" if self.os == "android" => "linux",
            "uwp" => "pc",
            "espressif" => "",
            _ if self.arch == "msp430" => "",
            vendor => vendor,
        };
        let os = match self.os {
            "macos" => "macosx",
            "visionos" => "xros",
            "uefi" => "windows",
            "solid_asp3" | "horizon" | "teeos" | "nuttx" | "espidf" => "none",
            "nto" => "unknown",    // FIXME
            "trusty" => "unknown", // FIXME
            os => os,
        };
        let version = version.unwrap_or("");
        let env = match self.env {
            "newlib" | "nto70" | "nto71" | "nto71_iosock" | "p1" | "p2" | "relibc" | "sgx"
            | "uclibc" => "",
            env => env,
        };
        let abi = match self.abi {
            "sim" => "simulator",
            "llvm" | "softfloat" | "uwp" | "vec-extabi" => "",
            "ilp32" => "_ilp32",
            "abi64" => "",
            abi => abi,
        };
        match (vendor, env, abi) {
            ("", "", "") => format!("{arch}-{os}{version}"),
            ("", env, abi) => format!("{arch}-{os}{version}-{env}{abi}"),
            (vendor, "", "") => format!("{arch}-{vendor}-{os}{version}"),
            (vendor, env, abi) => format!("{arch}-{vendor}-{os}{version}-{env}{abi}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use crate::TargetInfo;

    #[test]
    fn basic_llvm_triple_guessing() {
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "unknown",
                os: "linux",
                env: "",
                abi: "",
            }
            .llvm_target(None),
            "aarch64-unknown-linux"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "linux",
                env: "gnu",
                abi: "",
            }
            .llvm_target(None),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "linux",
                env: "gnu",
                abi: "eabi",
            }
            .llvm_target(None),
            "x86_64-unknown-linux-gnueabi"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "apple",
                os: "macos",
                env: "",
                abi: "",
            }
            .llvm_target(None),
            "x86_64-apple-macosx"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "ios",
                env: "",
                abi: "sim",
            }
            .llvm_target(Some("14.0")),
            "arm64-apple-ios14.0-simulator"
        );
    }

    #[test]
    fn llvm_for_all_rustc_targets() {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

        let target_list = Command::new(&rustc)
            .arg("--print=target-list")
            .output()
            .unwrap()
            .stdout;
        let target_list = String::from_utf8(target_list).unwrap();

        let mut has_failure = false;
        for target in target_list.lines() {
            let spec_json = Command::new(&rustc)
                .arg("--target")
                .arg(target)
                .arg("-Zunstable-options")
                .arg("--print=target-spec-json")
                .env("RUSTC_BOOTSTRAP", "1") // Crimes
                .output()
                .unwrap()
                .stdout;
            let spec_json = String::from_utf8(spec_json).unwrap();

            // JSON crimes
            let expected = spec_json
                .split_once("llvm-target\": \"")
                .unwrap()
                .1
                .split_once("\"")
                .unwrap()
                .0;
            let actual =
                TargetInfo::from_rustc_target(target).map(|target| target.llvm_target(None));

            if Some(expected) != actual.as_deref().ok() {
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
}
