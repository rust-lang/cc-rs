// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::process::Command;

/// Attempts to find a tool within an MSVC installation using the Windows
/// registry as a point to search from.
///
/// The `target` argument is the target that the tool should work for (e.g.
/// compile or link for) and the `tool` argument is the tool to find (e.g.
/// `cl.exe` or `link.exe`).
///
/// This function will return `None` if the tool could not be found, or it will
/// return `Some(cmd)` which represents a command that's ready to execute the
/// tool with the appropriate environment variables set.
///
/// Note that this function always returns `None` for non-MSVC targets.
#[cfg(not(windows))]
pub fn find(_target: &str, _tool: &str) -> Option<Command> {
    None
}

#[cfg(windows)]
pub fn find(target: &str, tool: &str) -> Option<Command> {
    use std::env;
    use std::ffi::OsString;
    use std::io;
    use std::fs;
    use std::path::{Path, PathBuf};
    use registry::{RegistryKey, LOCAL_MACHINE};

    if !target.contains("msvc") { return None }

    // When finding binaries the 32-bit version is at the top level but the
    // versions to cross to other architectures are stored in sub-folders.
    // Unknown architectures also just bail out early to return the standard
    // `link.exe` command.
    let extra = if target.starts_with("i686") {
        ""
    } else if target.starts_with("x86_64") {
        "amd64"
    } else if target.starts_with("arm") {
        "arm"
    } else {
        return None
    };

    let vs_install_dir = get_vs_install_dir();
    let mut path_to_add = None;

    // First up, we need to find the `link.exe` binary itself, and there's a few
    // locations that we can look. First up is the standard VCINSTALLDIR
    // environment variable which is normally set by the vcvarsall.bat file. If
    // an environment is set up manually by whomever's driving the compiler then
    // we shouldn't muck with that decision and should instead respect that.
    //
    // Finally we read the Windows registry to discover the VS install root.
    // From here we probe just to make sure that it exists.
    let mut cmd = env::var_os("VCINSTALLDIR").and_then(|dir| {
        let mut p = PathBuf::from(dir);
        p.push("bin");
        p.push(extra);
        let tool = p.join(tool);
        if fs::metadata(&tool).is_ok() {
            path_to_add = Some(p);
            Some(tool)
        } else {
            None
        }
    }).or_else(|| {
        env::var_os("PATH").and_then(|path| {
            env::split_paths(&path).map(|p| p.join(tool)).find(|path| {
                fs::metadata(path).is_ok()
            })
        })
    }).or_else(|| {
        vs_install_dir.as_ref().and_then(|p| {
            let mut p = p.join("VC/bin");
            p.push(extra);
            let tool = p.join(tool);
            if fs::metadata(&tool).is_ok() {
                path_to_add = Some(p);
                Some(tool)
            } else {
                None
            }
        })
    }).map(|tool| {
        Command::new(tool)
    }).unwrap_or_else(|| {
        Command::new(tool)
    });

    let mut paths = Vec::new();
    if let Some(path) = path_to_add {
        paths.push(path);
        if let Some(root) = get_windows_sdk_bin_path(target) {
            paths.push(root);
        }
    }
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    cmd.env("PATH", env::join_paths(&paths).unwrap());

    // The MSVC compiler uses the INCLUDE environment variable as the default
    // lookup path for headers. This environment variable is normally set up
    // by the VS shells, so we only want to start adding our own pieces if it's
    // not set.
    //
    // If we're adding our own pieces, then we need to add two primary
    // directories to the default search path for the linker. The first is in
    // the VS install direcotry and the next is the Windows SDK directory.
    if env::var_os("INCLUDE").is_none() {
        let mut includes = Vec::new();
        if let Some(ref vs_install_dir) = vs_install_dir {
            includes.push(vs_install_dir.join("VC/include"));
            if let Some((ucrt_root, vers)) = ucrt_install_dir(vs_install_dir) {
                includes.push(ucrt_root.join("Include").join(vers).join("ucrt"));
            }
        }
        if let Some((path, major)) = get_windows_sdk_path() {
            if major >= 8 {
                includes.push(path.join("include/shared"));
                includes.push(path.join("include/um"));
                includes.push(path.join("include/winrt"));
            } else {
                includes.push(path.join("include"));
            }
        } else if let Some(ref vs_install_dir) = vs_install_dir {
            includes.push(vs_install_dir.clone());
        }
        cmd.env("INCLUDE", env::join_paths(&includes).unwrap());
    }

    // Similarly with INCLUDE above, let's set LIB if it's not defined.
    if env::var_os("LIB").is_none() {
        let mut libs = Vec::new();
        if let Some(ref vs_install_dir) = vs_install_dir {
            libs.push(vs_install_dir.join("VC/lib").join(extra));
            if let Some((ucrt_root, vers)) = ucrt_install_dir(vs_install_dir) {
                if let Some(arch) = windows_sdk_v8_subdir(target) {
                    libs.push(ucrt_root.join("Lib").join(vers)
                                       .join("ucrt").join(arch));
                }
            }
        }
        if let Some(path) = get_windows_sdk_lib_path(target) {
            libs.push(path);
        }
        cmd.env("LIB", env::join_paths(&libs).unwrap());
    }

    return Some(cmd);

    // When looking for the Visual Studio installation directory we look in a
    // number of locations in varying degrees of precedence:
    //
    // 1. The Visual Studio registry keys
    // 2. The Visual Studio Express registry keys
    // 3. A number of somewhat standard environment variables
    //
    // If we find a hit from any of these keys then we strip off the IDE/Tools
    // folders which are typically found at the end.
    //
    // As a final note, when we take a look at the registry keys they're
    // typically found underneath the version of what's installed, but we don't
    // quite know what's installed. As a result we probe all sub-keys of the two
    // keys we're looking at to find out the maximum version of what's installed
    // and we use that root directory.
    fn get_vs_install_dir() -> Option<PathBuf> {
        LOCAL_MACHINE.open(r"SOFTWARE\Microsoft\VisualStudio".as_ref()).or_else(|_| {
            LOCAL_MACHINE.open(r"SOFTWARE\Microsoft\VCExpress".as_ref())
        }).ok().and_then(|key| {
            max_version(&key).and_then(|(_vers, key)| {
                key.query_str("InstallDir").ok()
            })
        }).or_else(|| {
            env::var_os("VS120COMNTOOLS")
        }).or_else(|| {
            env::var_os("VS100COMNTOOLS")
        }).or_else(|| {
            env::var_os("VS90COMNTOOLS")
        }).or_else(|| {
            env::var_os("VS80COMNTOOLS")
        }).map(PathBuf::from).and_then(|mut dir| {
            if dir.ends_with("Common7/IDE") || dir.ends_with("Common7/Tools") {
                dir.pop();
                dir.pop();
                Some(dir)
            } else {
                None
            }
        })
    }

    // Given a registry key, look at all the sub keys and find the one which has
    // the maximal numeric value.
    //
    // Returns the name of the maximal key as well as the opened maximal key.
    fn max_version(key: &RegistryKey) -> Option<(OsString, RegistryKey)> {
        let mut max_vers = 0;
        let mut max_key = None;
        for subkey in key.iter().filter_map(|k| k.ok()) {
            let val = subkey.to_str().and_then(|s| {
                s.trim_left_matches("v").replace(".", "").parse().ok()
            });
            let val = match val {
                Some(s) => s,
                None => continue,
            };
            if val > max_vers {
                if let Ok(k) = key.open(&subkey) {
                    max_vers = val;
                    max_key = Some((subkey, k));
                }
            }
        }
        return max_key
    }

    fn get_windows_sdk_path() -> Option<(PathBuf, usize)> {
        let key = r"SOFTWARE\Microsoft\Microsoft SDKs\Windows";
        let key = LOCAL_MACHINE.open(key.as_ref());
        let (n, k) = match key.ok().as_ref().and_then(max_version) {
            Some(p) => p,
            None => return None,
        };
        let mut parts = n.to_str().unwrap().trim_left_matches("v").splitn(2, ".");
        let major = parts.next().unwrap().parse::<usize>().unwrap();
        let _minor = parts.next().unwrap().parse::<usize>().unwrap();
        k.query_str("InstallationFolder").ok().map(|p| {
            (PathBuf::from(p), major)
        })
    }

    fn get_windows_sdk_lib_path(target: &str) -> Option<PathBuf> {
        let (mut root, major) = match get_windows_sdk_path() {
            Some(pair) => pair,
            None => return None,
        };
        root.push("Lib");
        if major <= 7 {
            // In Windows SDK 7.x, x86 libraries are directly in the Lib
            // folder, x64 libraries are inside, and it's not necessary to
            // link agains the SDK 7.x when targeting ARM or other
            // architectures.
            if target.starts_with("i686") {
                Some(root)
            } else if target.starts_with("x86_64") {
                Some(root.join("x64"))
            } else {
                None
            }
        } else {
            // Windows SDK 8.x installes libraries in a folder whose names
            // depend on the version of the OS you're targeting. By default
            // choose the newest, which usually corresponds to the version of
            // the OS you've installed the SDK on.
            let extra = match windows_sdk_v8_subdir(target) {
                Some(extra) => extra,
                None => return None,
            };
            ["winv6.3", "win8", "win7"].iter().map(|p| root.join(p)).find(|part| {
                fs::metadata(part).is_ok()
            }).map(|path| {
                path.join("um").join(extra)
            })
        }
    }

    fn get_windows_sdk_bin_path(target: &str) -> Option<PathBuf> {
        let (mut root, major) = match get_windows_sdk_path() {
            Some(pair) => pair,
            None => return None,
        };
        root.push("bin");
        if major <= 7 {
            None // untested path, not sure if this dir exists
        } else {
            root.push(match windows_sdk_v8_subdir(target) {
                Some(extra) => extra,
                None => return None,
            });
            if fs::metadata(&root).is_ok() {Some(root)} else {None}
        }
    }

    fn windows_sdk_v8_subdir(target: &str) -> Option<&'static str> {
        if target.starts_with("i686") {
            Some("x86")
        } else if target.starts_with("x86_64") {
            Some("x64")
        } else if target.starts_with("arm") {
            Some("arm")
        } else {
            None
        }
    }

    fn ucrt_install_dir(vs_install_dir: &Path) -> Option<(PathBuf, String)> {
        let is_vs_14 = vs_install_dir.iter().filter_map(|p| p.to_str()).any(|s| {
            s == "Microsoft Visual Studio 14.0"
        });
        if !is_vs_14 {
            return None
        }
        let key = r"SOFTWARE\Microsoft\Windows Kits\Installed Roots";
        let sdk_dir = LOCAL_MACHINE.open(key.as_ref()).and_then(|p| {
            p.query_str("KitsRoot10")
        }).map(PathBuf::from);
        let sdk_dir = match sdk_dir {
            Ok(p) => p,
            Err(..) => return None,
        };
        (move || -> io::Result<_> {
            let mut max = None;
            let mut max_s = None;
            for entry in try!(fs::read_dir(&sdk_dir.join("Lib"))) {
                let entry = try!(entry);
                if let Ok(s) = entry.file_name().into_string() {
                    if let Ok(u) = s.replace(".", "").parse::<usize>() {
                        if Some(u) > max {
                            max = Some(u);
                            max_s = Some(s);
                        }
                    }
                }
            }
            Ok(max_s.map(|m| (sdk_dir, m)))
        })().ok().and_then(|x| x)
    }
}
