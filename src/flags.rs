use crate::target::TargetInfo;
use crate::{Build, Error, ErrorKind, Tool, ToolFamily};
use std::borrow::Cow;
use std::ffi::OsString;

#[derive(Debug, PartialEq, Default)]
pub(crate) struct RustcCodegenFlags<'a> {
    branch_protection: Option<&'a str>,
    code_model: Option<&'a str>,
    no_vectorize_loops: bool,
    no_vectorize_slp: bool,
    profile_generate: Option<&'a str>,
    profile_use: Option<&'a str>,
    control_flow_guard: Option<&'a str>,
    lto: Option<&'a str>,
    relocation_model: Option<&'a str>,
    embed_bitcode: Option<bool>,
    force_frame_pointers: Option<bool>,
    link_dead_code: Option<bool>,
    no_redzone: Option<bool>,
    soft_float: Option<bool>,
}

impl<'this> RustcCodegenFlags<'this> {
    // Parse flags obtained from CARGO_ENCODED_RUSTFLAGS
    pub(crate) fn parse(rustflags_env: &'this str) -> Result<Self, Error> {
        fn is_flag_prefix(flag: &str) -> bool {
            [
                "-Z",
                "-C",
                "--codegen",
                "-L",
                "-l",
                "-o",
                "-W",
                "--warn",
                "-A",
                "--allow",
                "-D",
                "--deny",
                "-F",
                "--forbid",
            ]
            .contains(&flag)
        }

        fn handle_flag_prefix<'a>(prev: &'a str, curr: &'a str) -> (&'a str, &'a str) {
            match prev {
                "--codegen" | "-C" => ("-C", curr),
                // Handle flags passed like --codegen=code-model=small
                _ if curr.starts_with("--codegen=") => ("-C", &curr[10..]),
                "-Z" => ("-Z", curr),
                "-L" | "-l" | "-o" => (prev, curr),
                // Handle lint flags
                "-W" | "--warn" => ("-W", curr),
                "-A" | "--allow" => ("-A", curr),
                "-D" | "--deny" => ("-D", curr),
                "-F" | "--forbid" => ("-F", curr),
                _ => ("", curr),
            }
        }

        let mut codegen_flags = Self::default();

        let mut prev_prefix = None;
        for curr in rustflags_env.split("\u{1f}") {
            let prev = prev_prefix.take().unwrap_or("");
            if prev.is_empty() && is_flag_prefix(curr) {
                prev_prefix = Some(curr);
                continue;
            }

            let (prefix, rustc_flag) = handle_flag_prefix(prev, curr);
            codegen_flags.set_rustc_flag(prefix, rustc_flag)?;
        }

        Ok(codegen_flags)
    }

    fn set_rustc_flag(&mut self, prefix: &str, flag: &'this str) -> Result<(), Error> {
        // Convert a textual representation of a bool-like rustc flag argument into an actual bool
        fn arg_to_bool(arg: impl AsRef<str>) -> Option<bool> {
            match arg.as_ref() {
                "y" | "yes" | "on" | "true" => Some(true),
                "n" | "no" | "off" | "false" => Some(false),
                _ => None,
            }
        }

        let (flag, value) = if let Some((flag, value)) = flag.split_once('=') {
            (flag, Some(value))
        } else {
            (flag, None)
        };
        let flag = if prefix.is_empty() {
            Cow::Borrowed(flag)
        } else {
            Cow::Owned(format!("{prefix}{flag}"))
        };

        fn flag_ok_or<'flag>(
            flag: Option<&'flag str>,
            msg: &'static str,
        ) -> Result<&'flag str, Error> {
            flag.ok_or(Error::new(ErrorKind::InvalidFlag, msg))
        }

        match flag.as_ref() {
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#code-model
            "-Ccode-model" => {
                self.code_model = Some(flag_ok_or(value, "-Ccode-model must have a value")?);
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-vectorize-loops
            "-Cno-vectorize-loops" => self.no_vectorize_loops = true,
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-vectorize-slp
            "-Cno-vectorize-slp" => self.no_vectorize_slp = true,
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#profile-generate
            "-Cprofile-generate" => {
                self.profile_generate =
                    Some(flag_ok_or(value, "-Cprofile-generate must have a value")?);
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#profile-use
            "-Cprofile-use" => {
                self.profile_use = Some(flag_ok_or(value, "-Cprofile-use must have a value")?);
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#control-flow-guard
            "-Ccontrol-flow-guard" => self.control_flow_guard = value.or(Some("true")),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#lto
            "-Clto" => self.lto = value.or(Some("true")),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#relocation-model
            "-Crelocation-model" => {
                self.relocation_model =
                    Some(flag_ok_or(value, "-Crelocation-model must have a value")?);
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#embed-bitcode
            "-Cembed-bitcode" => self.embed_bitcode = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#force-frame-pointers
            "-Cforce-frame-pointers" => {
                self.force_frame_pointers = value.map_or(Some(true), arg_to_bool)
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#link-dead-code
            "-Clink-dead-code" => self.link_dead_code = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-redzone
            "-Cno-redzone" => self.no_redzone = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#soft-float
            // Note: This flag is now deprecated in rustc.
            "-Csoft-float" => self.soft_float = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/beta/unstable-book/compiler-flags/branch-protection.html
            // FIXME: Drop the -Z variant and update the doc link once the option is stabilised
            "-Zbranch-protection" | "-Cbranch-protection" => {
                self.branch_protection =
                    Some(flag_ok_or(value, "-Zbranch-protection must have a value")?);
            }
            _ => {}
        }
        Ok(())
    }

    // Rust and clang/cc don't agree on what equivalent flags should look like.
    pub(crate) fn cc_flags(&self, build: &Build, tool: &mut Tool, target: &TargetInfo<'_>) {
        let family = tool.family;
        // Push `flag` to `flags` if it is supported by the currently used CC
        let mut push_if_supported = |flag: OsString| {
            if build
                .is_flag_supported_inner(&flag, tool, target)
                .unwrap_or(false)
            {
                tool.args.push(flag);
            } else {
                build.cargo_output.print_warning(&format!(
                    "Inherited flag {:?} is not supported by the currently used CC",
                    flag
                ));
            }
        };

        let clang_or_gnu =
            matches!(family, ToolFamily::Clang { .. }) || matches!(family, ToolFamily::Gnu { .. });

        // Flags shared between clang and gnu
        if clang_or_gnu {
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mbranch-protection
            if let Some(value) = self.branch_protection {
                push_if_supported(
                    format!("-mbranch-protection={}", value.replace(",", "+")).into(),
                );
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mcmodel
            if let Some(value) = self.code_model {
                push_if_supported(format!("-mcmodel={value}").into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-vectorize
            if self.no_vectorize_loops {
                push_if_supported("-fno-vectorize".into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-slp-vectorize
            if self.no_vectorize_slp {
                push_if_supported("-fno-slp-vectorize".into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mguard
            if let Some(value) = self.control_flow_guard {
                let cc_val = match value {
                    "y" | "yes" | "on" | "true" | "checks" => Some("cf"),
                    "nochecks" => Some("cf-nochecks"),
                    "n" | "no" | "off" | "false" => Some("none"),
                    _ => None,
                };
                if let Some(cc_val) = cc_val {
                    push_if_supported(format!("-mguard={cc_val}").into());
                }
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-flto
            if let Some(value) = self.lto {
                let cc_val = match value {
                    "y" | "yes" | "on" | "true" | "fat" => Some("full"),
                    "thin" => Some("thin"),
                    _ => None,
                };
                if let Some(cc_val) = cc_val {
                    push_if_supported(format!("-flto={cc_val}").into());
                }
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fPIC
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fPIE
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mdynamic-no-pic
            if let Some(value) = self.relocation_model {
                let cc_flag = match value {
                    "pic" => Some("-fPIC"),
                    "pie" => Some("-fPIE"),
                    "dynamic-no-pic" => Some("-mdynamic-no-pic"),
                    _ => None,
                };
                if let Some(cc_flag) = cc_flag {
                    push_if_supported(cc_flag.into());
                }
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fembed-bitcode
            if let Some(value) = self.embed_bitcode {
                let cc_val = if value { "all" } else { "off" };
                push_if_supported(format!("-fembed-bitcode={cc_val}").into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-omit-frame-pointer
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fomit-frame-pointer
            if let Some(value) = self.force_frame_pointers {
                let cc_flag = if value {
                    "-fno-omit-frame-pointer"
                } else {
                    "-fomit-frame-pointer"
                };
                push_if_supported(cc_flag.into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-dead_strip
            if let Some(false) = self.link_dead_code {
                push_if_supported("-dead_strip".into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mno-red-zone
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mred-zone
            if let Some(value) = self.no_redzone {
                let cc_flag = if value { "-mno-red-zone" } else { "-mred-zone" };
                push_if_supported(cc_flag.into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-msoft-float
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mno-soft-float
            if let Some(value) = self.soft_float {
                let cc_flag = if value {
                    "-msoft-float"
                } else {
                    "-mno-soft-float"
                };
                push_if_supported(cc_flag.into());
            }
        }

        // Compiler-exclusive flags
        match family {
            ToolFamily::Clang { .. } => {
                // GNU and Clang compilers both support the same PGO flags, but they use different libraries and
                // different formats for the profile files which are not compatible.
                // clang and rustc both internally use llvm, so we want to inherit the PGO flags only for clang.
                // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fprofile-generate
                if let Some(value) = self.profile_generate {
                    push_if_supported(format!("-fprofile-generate={value}").into());
                }
                // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fprofile-use
                if let Some(value) = self.profile_use {
                    push_if_supported(format!("-fprofile-use={value}").into());
                }
            }
            ToolFamily::Gnu { .. } => {}
            ToolFamily::Msvc { .. } => {
                // https://learn.microsoft.com/en-us/cpp/build/reference/guard-enable-control-flow-guard
                if let Some(value) = self.control_flow_guard {
                    let cc_val = match value {
                        "y" | "yes" | "on" | "true" | "checks" => Some("cf"),
                        "n" | "no" | "off" | "false" => Some("cf-"),
                        _ => None,
                    };
                    if let Some(cc_val) = cc_val {
                        push_if_supported(format!("/guard:{cc_val}").into());
                    }
                }
                // https://learn.microsoft.com/en-us/cpp/build/reference/oy-frame-pointer-omission
                if let Some(value) = self.force_frame_pointers {
                    // Flag is unsupported on 64-bit arches
                    if !target.arch.contains("64") {
                        let cc_flag = if value { "/Oy-" } else { "/Oy" };
                        push_if_supported(cc_flag.into());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn check(env: &str, expected: &RustcCodegenFlags) {
        let actual = RustcCodegenFlags::parse(env).unwrap();
        assert_eq!(actual, *expected);
    }

    #[test]
    fn codegen_type() {
        let expected = RustcCodegenFlags {
            code_model: Some("tiny"),
            ..RustcCodegenFlags::default()
        };
        check("-Ccode-model=tiny", &expected);
        check("-C\u{1f}code-model=tiny", &expected);
        check("--codegen\u{1f}code-model=tiny", &expected);
        check("--codegen=code-model=tiny", &expected);
    }

    #[test]
    fn precedence() {
        check(
            "-ccode-model=tiny\u{1f}-Ccode-model=small",
            &RustcCodegenFlags {
                code_model: Some("small"),
                ..RustcCodegenFlags::default()
            },
        );
    }

    #[test]
    fn two_valid_prefixes() {
        let expected = RustcCodegenFlags::default();
        check("-L\u{1f}-Clto", &expected);
    }

    #[test]
    fn three_valid_prefixes() {
        let expected = RustcCodegenFlags {
            lto: Some("true"),
            ..RustcCodegenFlags::default()
        };
        check("-L\u{1f}-L\u{1f}-Clto", &expected);
    }

    #[test]
    fn all_rustc_flags() {
        // Throw all possible flags at the parser to catch false positives
        let flags = [
            // Set all the flags we recognise first
            "-Ccode-model=tiny",
            "-Ccontrol-flow-guard=yes",
            "-Cembed-bitcode=no",
            "-Cforce-frame-pointers=yes",
            "-Clto=false",
            "-Clink-dead-code=yes",
            "-Cno-redzone=yes",
            "-Cno-vectorize-loops",
            "-Cno-vectorize-slp",
            "-Cprofile-generate=fooprofile",
            "-Cprofile-use=fooprofile",
            "-Crelocation-model=pic",
            "-Csoft-float=yes",
            "-Zbranch-protection=bti,pac-ret,leaf",
            // Set flags we don't recognise but rustc supports next
            // rustc flags
            "--cfg",
            "a",
            "--check-cfg 'cfg(verbose)",
            "-L",
            "/usr/lib/foo",
            "-l",
            "static:+whole-archive=mylib",
            "--crate-type=dylib",
            "--crate-name=foo",
            "--edition=2021",
            "--emit=asm",
            "--print=crate-name",
            "-g",
            "-O",
            "-o",
            "foooutput",
            "--out-dir",
            "foooutdir",
            "--target",
            "aarch64-unknown-linux-gnu",
            "-W",
            "missing-docs",
            "-D",
            "unused-variables",
            "--force-warn",
            "dead-code",
            "-A",
            "unused",
            "-F",
            "unused",
            "--cap-lints",
            "warn",
            "--version",
            "--verbose",
            "-v",
            "--extern",
            "foocrate",
            "--sysroot",
            "fooroot",
            "--error-format",
            "human",
            "--color",
            "auto",
            "--diagnostic-width",
            "80",
            "--remap-path-prefix",
            "foo=bar",
            "--json=artifact",
            // Codegen flags
            "-Car",
            "-Ccodegen-units=1",
            "-Ccollapse-macro-debuginfo=yes",
            "-Cdebug-assertions=yes",
            "-Cdebuginfo=1",
            "-Cdefault-linker-libraries=yes",
            "-Cdlltool=foo",
            "-Cextra-filename=foo",
            "-Cforce-unwind-tables=yes",
            "-Cincremental=foodir",
            "-Cinline-threshold=6",
            "-Cinstrument-coverage",
            "-Clink-arg=-foo",
            "-Clink-args=-foo",
            "-Clink-self-contained=yes",
            "-Clinker=lld",
            "-Clinker-flavor=ld.lld",
            "-Clinker-plugin-lto=yes",
            "-Cllvm-args=foo",
            "-Cmetadata=foo",
            "-Cno-prepopulate-passes",
            "-Cno-stack-check",
            "-Copt-level=3",
            "-Coverflow-checks=yes",
            "-Cpanic=abort",
            "-Cpasses=foopass",
            "-Cprefer-dynamic=yes",
            "-Crelro-level=partial",
            "-Cremark=all",
            "-Crpath=yes",
            "-Csave-temps=yes",
            "-Csplit-debuginfo=packed",
            "-Cstrip=symbols",
            "-Csymbol-mangling-version=v0",
            "-Ctarget-cpu=native",
            "-Ctarget-feature=+sve",
            // Unstable options
            "-Ztune-cpu=machine",
        ];
        check(
            &flags.join("\u{1f}"),
            &RustcCodegenFlags {
                code_model: Some("tiny"),
                control_flow_guard: Some("yes"),
                embed_bitcode: Some(false),
                force_frame_pointers: Some(true),
                link_dead_code: Some(true),
                lto: Some("false"),
                no_redzone: Some(true),
                no_vectorize_loops: true,
                no_vectorize_slp: true,
                profile_generate: Some("fooprofile"),
                profile_use: Some("fooprofile"),
                relocation_model: Some("pic"),
                soft_float: Some(true),
                branch_protection: Some("bti,pac-ret,leaf"),
            },
        );
    }
}
