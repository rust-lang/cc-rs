#![cfg_attr(test, allow(dead_code))]

use std::env;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

trait EnumUnwrapExt {
    type Value;

    fn expect_fmt(self, args: fmt::Arguments<'_>) -> Self::Value;
}

/// This is a separate function to reduce the code size of the methods
///
/// Adapted from libstd
/// https://doc.rust-lang.org/src/core/result.rs.html#1785-1792
#[inline(never)]
#[cold]
#[track_caller]
fn unwrap_failed(msg: fmt::Arguments<'_>, error: &dyn fmt::Debug) -> ! {
    panic!("{}: {:?}", msg, error)
}

impl<T, E> EnumUnwrapExt for Result<T, E>
where
    E: fmt::Debug,
{
    type Value = T;

    /// Adapted from libstd
    /// https://doc.rust-lang.org/src/core/result.rs.html#1064-1066
    #[inline]
    #[track_caller]
    fn expect_fmt(self, msg: fmt::Arguments<'_>) -> T {
        match self {
            Ok(t) => t,
            Err(e) => unwrap_failed(msg, &e),
        }
    }
}

/// This is a separate function to reduce the code size of .expect() itself.
///
/// Adapted from libstd
/// https://doc.rust-lang.org/src/core/option.rs.html#1872-1880
#[inline(never)]
#[cold]
#[track_caller]
fn expect_failed(msg: fmt::Arguments<'_>) -> ! {
    panic!("{}", msg)
}

impl<T> EnumUnwrapExt for Option<T> {
    type Value = T;

    /// Adapted from libstd
    /// https://doc.rust-lang.org/src/core/option.rs.html#734-743
    #[inline]
    #[track_caller]
    fn expect_fmt(self, msg: fmt::Arguments<'_>) -> T {
        match self {
            Some(val) => val,
            None => expect_failed(msg),
        }
    }
}

fn main() {
    let mut args = env::args();
    let program = args.next().expect("Unexpected empty args");

    let out_dir = PathBuf::from(
        env::var_os("GCCTEST_OUT_DIR")
            .expect_fmt(format_args!("{}: GCCTEST_OUT_DIR not found", program)),
    );

    // Find the first nonexistent candidate file to which the program's args can be written.
    for i in 0.. {
        let candidate = &out_dir.join(format!("out{}", i));

        // If the file exists, commands have already run. Try again.
        if candidate.exists() {
            continue;
        }

        // Create a file and record the args passed to the command.
        let mut f = File::create(candidate).expect_fmt(format_args!(
            "{}: can't create candidate: {}",
            program,
            candidate.display()
        ));
        for arg in args {
            writeln!(f, "{}", arg).expect_fmt(format_args!(
                "{}: can't write to candidate: {}",
                program,
                candidate.display()
            ));
        }
        break;
    }

    // Create a file used by some tests.
    let path = &out_dir.join("libfoo.a");
    File::create(path).expect_fmt(format_args!(
        "{}: can't create libfoo.a: {}",
        program,
        path.display()
    ));
}
