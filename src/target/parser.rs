use std::env;

use crate::{target::TargetInfo, utilities::OnceLock, Error, ErrorKind};

#[derive(Debug)]
struct TargetInfoParserInner {
    full_arch: Box<str>,
    arch: Box<str>,
    vendor: Box<str>,
    os: Box<str>,
    env: Box<str>,
    abi: Box<str>,
}

impl TargetInfoParserInner {
    fn from_cargo_environment_variables() -> Result<Self, Error> {
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

        let cargo_env = |name, fallback: Option<&str>| -> Result<Box<str>, Error> {
            // No need to emit `rerun-if-env-changed` for these,
            // as they are controlled by Cargo itself.
            #[allow(clippy::disallowed_methods)]
            match env::var(name) {
                Ok(var) => Ok(var.into_boxed_str()),
                Err(err) => match fallback {
                    Some(fallback) => Ok(fallback.into()),
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
        let fallback_target = TargetInfo::from_rustc_target(&target_triple).ok();
        let ft = fallback_target.as_ref();
        let arch = cargo_env("CARGO_CFG_TARGET_ARCH", ft.map(|t| t.arch))?;
        let vendor = cargo_env("CARGO_CFG_TARGET_VENDOR", ft.map(|t| t.vendor))?;
        let os = cargo_env("CARGO_CFG_TARGET_OS", ft.map(|t| t.os))?;
        let env = cargo_env("CARGO_CFG_TARGET_ENV", ft.map(|t| t.env))?;
        // `target_abi` was stabilized in Rust 1.78, which is higher than our
        // MSRV, so it may not always be available; In that case, fall back to
        // `""`, which is _probably_ correct for unknown target triples.
        let abi = cargo_env("CARGO_CFG_TARGET_ABI", ft.map(|t| t.abi))
            .unwrap_or_else(|_| String::default().into_boxed_str());

        Ok(Self {
            full_arch: full_arch.to_string().into_boxed_str(),
            arch,
            vendor,
            os,
            env,
            abi,
        })
    }
}

/// Parser for [`TargetInfo`], contains cached information.
#[derive(Default, Debug)]
pub(crate) struct TargetInfoParser(OnceLock<Result<TargetInfoParserInner, Error>>);

impl TargetInfoParser {
    pub fn parse_from_cargo_environment_variables(&self) -> Result<TargetInfo<'_>, Error> {
        match self
            .0
            .get_or_init(TargetInfoParserInner::from_cargo_environment_variables)
        {
            Ok(TargetInfoParserInner {
                full_arch,
                arch,
                vendor,
                os,
                env,
                abi,
            }) => Ok(TargetInfo {
                full_arch,
                arch,
                vendor,
                os,
                env,
                abi,
            }),
            Err(e) => Err(e.clone()),
        }
    }
}

/// Parse the full architecture into the simpler `cfg(target_arch = "...")`
/// that `rustc` exposes.
fn parse_arch(full_arch: &str) -> Option<&str> {
    // NOTE: Some of these don't necessarily match an existing target in
    // `rustc`. They're parsed anyhow to be as forward-compatible as possible,
    // while still being correct.
    //
    // See also:
    // https://docs.rs/cfg-expr/0.18.0/cfg_expr/targets/index.html
    // https://docs.rs/target-lexicon/0.13.2/target_lexicon/enum.Architecture.html
    // https://gcc.gnu.org/onlinedocs/gcc/Submodel-Options.html
    // `clang -print-targets`
    Some(match full_arch {
        arch if arch.starts_with("mipsisa32r6") => "mips32r6", // mipsisa32r6 | mipsisa32r6el
        arch if arch.starts_with("mipsisa64r6") => "mips64r6", // mipsisa64r6 | mipsisa64r6el

        arch if arch.starts_with("mips64") => "mips64", // mips64 | mips64el
        arch if arch.starts_with("mips") => "mips",     // mips | mipsel

        arch if arch.starts_with("loongarch64") => "loongarch64",
        arch if arch.starts_with("loongarch32") => "loongarch32",

        arch if arch.starts_with("powerpc64") => "powerpc64", // powerpc64 | powerpc64le
        arch if arch.starts_with("powerpc") => "powerpc",
        arch if arch.starts_with("ppc64") => "powerpc64",
        arch if arch.starts_with("ppc") => "powerpc",

        arch if arch.starts_with("x86_64") => "x86_64", // x86_64 | x86_64h
        arch if arch.starts_with("i") && arch.ends_with("86") => "x86", // i386 | i586 | i686

        "arm64ec" => "arm64ec", // https://github.com/rust-lang/rust/issues/131172
        arch if arch.starts_with("aarch64") => "aarch64", // arm64e | arm64_32
        arch if arch.starts_with("arm64") => "aarch64", // aarch64 | aarch64_be

        arch if arch.starts_with("arm") => "arm", // arm | armv7s | armeb | ...
        arch if arch.starts_with("thumb") => "arm", // thumbv4t | thumbv7a | thumbv8m | ...

        arch if arch.starts_with("riscv64") => "riscv64",
        arch if arch.starts_with("riscv32") => "riscv32",

        arch if arch.starts_with("wasm64") => "wasm64",
        arch if arch.starts_with("wasm32") => "wasm32", // wasm32 | wasm32v1
        "asmjs" => "wasm32",

        arch if arch.starts_with("nvptx64") => "nvptx64",
        arch if arch.starts_with("nvptx") => "nvptx",

        arch if arch.starts_with("bpf") => "bpf", // bpfeb | bpfel

        // https://github.com/bytecodealliance/wasmtime/tree/v30.0.1/pulley
        arch if arch.starts_with("pulley64") => "pulley64",
        arch if arch.starts_with("pulley32") => "pulley32",

        // https://github.com/Clever-ISA/Clever-ISA
        arch if arch.starts_with("clever") => "clever",

        "sparc" | "sparcv7" | "sparcv8" => "sparc",
        "sparc64" | "sparcv9" => "sparc64",

        "amdgcn" => "amdgpu",
        "avr" => "avr",
        "csky" => "csky",
        "hexagon" => "hexagon",
        "m68k" => "m68k",
        "msp430" => "msp430",
        "r600" => "r600",
        "s390x" => "s390x",
        "xtensa" => "xtensa",

        _ => return None,
    })
}

/// Parse environment (`cfg(target_env)`) and ABI (`cfg(target_abi)`) from
/// the last component of the target triple.
fn parse_envabi(last_component: &str) -> Option<(&str, &str)> {
    let (env, abi) = match last_component {
        // Combined environment and ABI

        // gnullvm | gnueabi | gnueabihf | gnuabiv2 | gnuabi64 | gnuspe | gnux32 | gnu_ilp32
        env_and_abi if env_and_abi.starts_with("gnu") => {
            let abi = env_and_abi.strip_prefix("gnu").unwrap();
            let abi = abi.strip_prefix("_").unwrap_or(abi);
            ("gnu", abi)
        }
        // musl | musleabi | musleabihf | muslabi64 | muslspe
        env_and_abi if env_and_abi.starts_with("musl") => {
            ("musl", env_and_abi.strip_prefix("musl").unwrap())
        }
        // uclibc | uclibceabi | uclibceabihf
        env_and_abi if env_and_abi.starts_with("uclibc") => {
            ("uclibc", env_and_abi.strip_prefix("uclibc").unwrap())
        }
        // newlib | newlibeabihf
        env_and_abi if env_and_abi.starts_with("newlib") => {
            ("newlib", env_and_abi.strip_prefix("newlib").unwrap())
        }

        // Environments
        "msvc" => ("msvc", ""),
        "ohos" => ("ohos", ""),
        "qnx700" => ("nto70", ""),
        "qnx710_iosock" => ("nto71_iosock", ""),
        "qnx710" => ("nto71", ""),
        "qnx800" => ("nto80", ""),
        "sgx" => ("sgx", ""),
        "threads" => ("threads", ""),

        // ABIs
        "abi64" => ("", "abi64"),
        "abiv2" => ("", "spe"),
        "eabi" => ("", "eabi"),
        "eabihf" => ("", "eabihf"),
        "macabi" => ("", "macabi"),
        "sim" => ("", "sim"),
        "softfloat" => ("", "softfloat"),
        "spe" => ("", "spe"),
        "x32" => ("", "x32"),

        // Badly named targets, ELF is already known from target OS.
        // Probably too late to fix now though.
        "elf" => ("", ""),
        // Undesirable to expose to user code (yet):
        // https://github.com/rust-lang/rust/pull/131166#issuecomment-2389541917
        "freestanding" => ("", ""),

        _ => return None,
    };
    Some((env, abi))
}

impl<'a> TargetInfo<'a> {
    pub(crate) fn from_rustc_target(target: &'a str) -> Result<Self, Error> {
        // FIXME(madsmtm): This target should be renamed, cannot be parsed
        // with the means we do below (since `none` must not be interpreted
        // as an env/ABI).
        if target == "x86_64-unknown-linux-none" {
            return Ok(Self {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "linux",
                env: "",
                abi: "",
            });
        }

        let mut components = target.split('-');

        // Insist that the triple contains at least a valid architecture.
        let full_arch = components.next().ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target}` was empty"),
        ))?;
        let arch = parse_arch(full_arch).ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownTarget,
                format!("target `{target}` had an unknown architecture"),
            )
        })?;

        // Newer target triples have begun omitting the vendor.
        // Additionally, some Linux distributions want to set their name as
        // the target vendor (so we have to assume that it can be an arbitary
        // string).
        //
        // To handle this, we parse the rest of the components from the BACK
        // instead, e.g. first the environment/abi (if present), then the OS,
        // and finally the vendor (if present).
        let mut components = components.rev();

        let envabi_or_os = components.next().ok_or(Error::new(
            ErrorKind::InvalidTarget,
            format!("target `{target}` must have at least two components"),
        ))?;

        // Unknown; assume instead that the last component is the OS name.
        let (os, mut env, mut abi, has_envabi) = {
            if let Some((env, abi)) = parse_envabi(envabi_or_os) {
                let os = components.next().ok_or(Error::new(
                    ErrorKind::InvalidTarget,
                    format!("target `{target}` must have an OS component"),
                ))?;
                (os, env, abi, true)
            } else {
                // Value did not contain env/ABI, so assume it is an OS instead.
                (envabi_or_os, "", "", false)
            }
        };

        // Various environment/ABIs are determined based on OS name.
        match os {
            "3ds" => env = "newlib",
            "vxworks" => env = "gnu",
            "rtems" => env = "newlib",
            "espidf" => env = "newlib",
            "redox" => env = "relibc",
            "aix" => abi = "vec-extabi",
            _ => {}
        }

        // Extra overrides for badly named targets.
        match target {
            // Actually simulator targets.
            "i386-apple-ios" | "i686-apple-ios" | "x86_64-apple-ios" | "x86_64-apple-tvos" => {
                abi = "sim";
            }
            // Name should've contained `muslabi64`.
            "mips64-openwrt-linux-musl" => {
                abi = "abi64";
            }
            // Specifies abi even though not in name.
            "armv6-unknown-freebsd" | "armv6k-nintendo-3ds" | "armv7-unknown-freebsd" => {
                abi = "eabihf";
            }
            // Specifies abi even though not in name.
            "armv7-unknown-linux-ohos" | "armv7-unknown-trusty" => {
                abi = "eabi";
            }
            // Specifies abi even though not in name.
            "riscv32e-unknown-none-elf"
            | "riscv32em-unknown-none-elf"
            | "riscv32emc-unknown-none-elf" => {
                abi = "ilp32e";
            }
            _ => {}
        }

        let os = match os {
            // Horizon is the common/internal OS name for 3DS and the Switch.
            "3ds" | "switch" => "horizon",
            // FIXME(madsmtm): macOS targets are badly named.
            "darwin" => "macos",

            // WASI targets contain the preview version in them too. Should've
            // been `wasi-p1`/`wasi-p2`, but that's probably too late now.
            os if os.starts_with("wasi") => {
                env = os.strip_prefix("wasi").unwrap();
                "wasi"
            }
            // FIXME(madsmtm): Badly named targets `*-linux-androideabi`,
            // should be `*-android-eabi`.
            "androideabi" => {
                abi = "eabi";
                "android"
            }

            os => os,
        };

        let vendor = match components.next().unwrap_or("unknown") {
            // esp, esp32, esp32s2 etc.
            vendor if vendor.starts_with("esp") => "espressif",
            // FIXME(madsmtm): https://github.com/rust-lang/rust/issues/131165
            "openwrt" => "unknown",
            // FIXME(madsmtm): Badly named targets `*-linux-android*`,
            // "linux" makes no sense as the vendor name.
            "linux" if os == "android" || os == "androideabi" => "unknown",
            vendor => vendor,
        };

        // FIXME(madsmtm): Unclear why both vendor and ABI is set on these.
        if matches!(vendor, "uwp" | "fortanix") {
            abi = vendor;
        }

        if components.next().is_some() {
            return Err(if has_envabi || components.next().is_some() {
                Error::new(
                    ErrorKind::InvalidTarget,
                    format!("too many components in target `{target}`"),
                )
            } else {
                Error::new(
                    ErrorKind::UnknownTarget,
                    format!("unknown environment/ABI `{envabi_or_os}` in target `{target}`"),
                )
            });
        }

        Ok(Self {
            full_arch,
            arch,
            vendor,
            os,
            env,
            abi,
        })
    }
}
