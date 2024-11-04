use std::ffi::OsString;

use super::TargetInfo;

impl TargetInfo<'_> {
    /// The `-m` family of compiler flags for GNU-like compilers. See:
    /// <https://gcc.gnu.org/onlinedocs/gcc/Submodel-Options.html>
    ///
    /// It is important that we pass as much information about the
    /// architecture here as possible because:
    /// 1. GCC is able to "cross"-compile to similar target architecture that
    ///    it was configured for (e.g. `x86` -> `x86_64` or `arm7` -> `arm6`),
    ///    and that process becomes smoother then.
    /// 2. Rust may have different defaults, and we want to get as close to
    ///    ABI compatibility as possible for soundness.
    pub fn gnu_arch_flags(&self) -> Vec<OsString> {
        let mut flags = vec![];

        match self.arch {
            // https://gcc.gnu.org/onlinedocs/gcc/AArch64-Options.html
            "aarch64" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/ARM-Options.html
            "arm" => {
                // armv7 selfs get to use armv7 instructions
                if (self.full_arch.starts_with("armv7") || self.full_arch.starts_with("thumbv7"))
                    && (self.os == "linux" || self.vendor == "kmc")
                {
                    flags.push("-march=armv7-a".into());

                    if self.abi == "eabihf" {
                        // lowest common denominator FPU
                        flags.push("-mfpu=vfpv3-d16".into());
                    }
                }

                // (x86 Android doesn't say "eabi")
                if self.os == "android" && self.full_arch.contains("v7") {
                    flags.push("-march=armv7-a".into());
                    flags.push("-mthumb".into());
                    if !self.full_arch.contains("neon") {
                        // On android we can guarantee some extra float instructions
                        // (specified in the android spec online)
                        // NEON guarantees even more; see below.
                        flags.push("-mfpu=vfpv3-d16".into());
                    }
                    flags.push("-mfloat-abi=softfp".into());
                }

                if self.full_arch.contains("neon") {
                    flags.push("-mfpu=neon-vfpv4".into());
                }

                if self.full_arch == "armv4t" && self.os == "linux" {
                    flags.push("-march=armv4t".into());
                    flags.push("-marm".into());
                    flags.push("-mfloat-abi=soft".into());
                }

                if self.full_arch == "armv5te" && self.os == "linux" {
                    flags.push("-march=armv5te".into());
                    flags.push("-marm".into());
                    flags.push("-mfloat-abi=soft".into());
                }

                // For us arm == armv6 by default
                if self.full_arch == "arm" && self.os == "linux" {
                    flags.push("-march=armv6".into());
                    flags.push("-marm".into());
                    if self.abi == "eabihf" {
                        flags.push("-mfpu=vfp".into());
                    } else {
                        flags.push("-mfloat-abi=soft".into());
                    }
                }

                // We can guarantee some settings for FRC
                if self.vendor == "frc" {
                    flags.push("-march=armv7-a".into());
                    flags.push("-mcpu=cortex-a9".into());
                    flags.push("-mfpu=vfpv3".into());
                    flags.push("-mfloat-abi=softfp".into());
                    flags.push("-marm".into());
                }

                if self.os == "none" && self.abi == "eabihf" {
                    flags.push("-mfloat-abi=hard".into())
                }
                if self.full_arch.starts_with("thumb") {
                    flags.push("-mthumb".into());
                }
                if self.full_arch.starts_with("thumbv6m") {
                    flags.push("-march=armv6s-m".into());
                }
                if self.full_arch.starts_with("thumbv7em") {
                    flags.push("-march=armv7e-m".into());

                    if self.abi == "eabihf" {
                        flags.push("-mfpu=fpv4-sp-d16".into())
                    }
                }
                if self.full_arch.starts_with("thumbv7m") {
                    flags.push("-march=armv7-m".into());
                }
                if self.full_arch.starts_with("thumbv8m.base") {
                    flags.push("-march=armv8-m.base".into());
                }
                if self.full_arch.starts_with("thumbv8m.main") {
                    flags.push("-march=armv8-m.main".into());

                    if self.abi == "eabihf" {
                        flags.push("-mfpu=fpv5-sp-d16".into())
                    }
                }
                if self.full_arch.starts_with("armebv7r") | self.full_arch.starts_with("armv7r") {
                    if self.full_arch.starts_with("armeb") {
                        flags.push("-mbig-endian".into());
                    } else {
                        flags.push("-mlittle-endian".into());
                    }

                    // ARM mode
                    flags.push("-marm".into());

                    // R Profile
                    flags.push("-march=armv7-r".into());

                    if self.abi == "eabihf" {
                        // lowest common denominator FPU
                        // (see Cortex-R4 technical reference manual)
                        flags.push("-mfpu=vfpv3-d16".into())
                    }
                }
                if self.full_arch.starts_with("armv7a") {
                    flags.push("-march=armv7-a".into());

                    if self.abi == "eabihf" {
                        // lowest common denominator FPU
                        flags.push("-mfpu=vfpv3-d16".into());
                    }
                }
            }
            // https://gcc.gnu.org/onlinedocs/gcc/AVR-Options.html
            "avr" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/eBPF-Options.html
            "bpf" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/C-SKY-Options.html
            "csky" => {}
            // TODO
            "hexagon" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/LoongArch-Options.html
            "loongarch64" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/M680x0-Options.html
            "m68k" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/MIPS-Options.html
            "mips" | "mips64" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/MSP430-Options.html
            "msp430" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/Nvidia-PTX-Options.html
            "nvptx64" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/RS_002f6000-and-PowerPC-Options.html
            "powerpc" | "powerpc64" => {
                if self.arch == "powerpc64" {
                    flags.push("-m64".into());
                }
            }
            // https://gcc.gnu.org/onlinedocs/gcc/RISC-V-Options.html
            "riscv32" | "riscv64" => {
                // get the 32i/32imac/32imc/64gc/64imac/... part
                let arch = &self.full_arch[5..];
                if arch.starts_with("64") {
                    if matches!(self.os, "linux" | "freebsd" | "netbsd") {
                        flags.push(("-march=rv64gc").into());
                        flags.push("-mabi=lp64d".into());
                    } else {
                        flags.push(("-march=rv".to_owned() + arch).into());
                        flags.push("-mabi=lp64".into());
                    }
                } else if arch.starts_with("32") {
                    if self.os == "linux" {
                        flags.push(("-march=rv32gc").into());
                        flags.push("-mabi=ilp32d".into());
                    } else {
                        flags.push(("-march=rv".to_owned() + arch).into());
                        flags.push("-mabi=ilp32".into());
                    }
                } else {
                    flags.push("-mcmodel=medany".into());
                }
            }
            // https://gcc.gnu.org/onlinedocs/gcc/S_002f390-and-zSeries-Options.html
            "s390x" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/SPARC-Options.html
            "sparc" | "sparc64" => {}
            // Currently unsupported by GCC
            "wasm32" | "wasm64" => {}
            // https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html
            "x86" | "x86_64" => {
                if self.arch == "x86" {
                    flags.push("-m32".into());
                } else if self.abi == "x32" {
                    flags.push("-mx32".into());
                } else {
                    flags.push("-m64".into());
                }

                // Turn codegen down on i586 to avoid some instructions.
                if self.full_arch == "i586" && self.os == "linux" {
                    flags.push("-march=pentium".into());
                }

                // Set codegen level for i686 correctly
                if self.full_arch == "i686" && self.os == "linux" {
                    flags.push("-march=i686".into());
                }

                // Looks like `musl-gcc` makes it hard for `-m32` to make its way
                // all the way to the linker, so we need to actually instruct the
                // linker that we're generating 32-bit executables as well. This'll
                // typically only be used for build scripts which transitively use
                // these flags that try to compile executables.
                if self.arch == "x86" && self.env == "musl" {
                    flags.push("-Wl,-melf_i386".into());
                }
            }
            // https://gcc.gnu.org/onlinedocs/gcc/Xtensa-Options.html
            "xtensa" => {}
            _arch => {
                // Silently allow unknown architectures, in case some users
                // compile with custom target json's on officially unsupported
                // platforms.
            }
        }

        flags
    }
}
