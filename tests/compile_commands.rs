#![allow(missing_docs)]

use std::{
    env,
    ffi::{OsStr, OsString},
    fmt::Write as _,
    io::Write as _,
    panic::{RefUnwindSafe, UnwindSafe},
    path::Path,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use cc::compile_commands::{json_compilation_database, CompileCommand, CompileCommandCollector};

use crate::support::Test;

mod support;

/// The command the shim recorded for `src`: the program, then its arguments.
#[track_caller]
fn recorded_command(test: &Test, src: &str) -> Vec<OsString> {
    let execution = test.cmd_for_source(src);
    std::iter::once(execution.program.into_os_string())
        .chain(execution.args.into_iter().map(OsString::from))
        .collect()
}

/// The program and arguments of `command`.
fn arguments(command: &CompileCommand) -> Vec<OsString> {
    command.arguments().map(OsStr::to_os_string).collect()
}

/// The one collected command that compiles `src`.
#[track_caller]
fn command_for<'a>(commands: &'a [CompileCommand], src: &str) -> &'a CompileCommand {
    let file = env::current_dir().unwrap().join(src);
    let matching: Vec<_> = commands.iter().filter(|c| c.file() == file).collect();
    assert_eq!(matching.len(), 1, "{src}: {commands:#?}");
    matching[0]
}

#[test]
fn compile_commands_auto_traits() {
    fn assert_auto_traits<T: Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}
    assert_auto_traits::<CompileCommand>();
    assert_auto_traits::<CompileCommandCollector>();
}

fn gnu_build(test: &Test, collector: &Arc<CompileCommandCollector>) -> cc::Build {
    let mut build = test.gcc();
    build
        .target("x86_64-unknown-linux-gnu")
        .host("x86_64-unknown-linux-gnu")
        .message_logger(Some(collector.clone()));
    build
}

/// Each object gets one entry, holding the command cc ran for it. With the
/// `parallel` feature this goes through the parallel path.
#[test]
fn compile_commands_are_the_commands_cc_runs() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    gnu_build(&test, &collector)
        .file("foo.c")
        .file("bar.c")
        .file("baz.S")
        .define("GREETING", "\"hello world\"")
        .try_compile("foo")
        .unwrap();

    let cwd = env::current_dir().unwrap();
    let commands: Vec<_> = collector.commands().collect();
    assert_eq!(commands.len(), 3, "{commands:#?}");
    for src in ["foo.c", "bar.c", "baz.S"] {
        let command = command_for(&commands, src);
        let args = arguments(command);
        assert_eq!(args, recorded_command(&test, src));
        assert_eq!(command.directory(), cwd);
        let o = args.iter().position(|arg| arg == "-o").unwrap();
        assert_eq!(command.output(), Path::new(&args[o + 1]));
        assert!(command.output().starts_with(test.td.path()), "{command:#?}");
    }
}

/// `.asm` files that MSVC targets assemble with MASM are recorded with the
/// assembler's command.
#[test]
fn compile_commands_include_msvc_assembler() {
    let mut test = Test::msvc();
    test.shim("masm-wrapper");
    test.env.set("CC_MASM_ASM", "masm-wrapper --from-env");
    let collector = Arc::new(CompileCommandCollector::new());
    test.gcc()
        .file("foo.c")
        .file("bar.asm")
        .message_logger(Some(collector.clone()))
        .try_compile("foo")
        .unwrap();

    let commands: Vec<_> = collector.commands().collect();
    assert_eq!(commands.len(), 2, "{commands:#?}");
    for (src, program) in [("foo.c", "cl"), ("bar.asm", "masm-wrapper")] {
        let command = command_for(&commands, src);
        test.cmd_for_source(src).must_run(program);
        assert_eq!(arguments(command), recorded_command(&test, src));
        let mut output = OsString::from("-Fo");
        output.push(command.output());
        assert!(
            command.arguments().any(|arg| arg == &*output),
            "{command:#?}"
        );
    }
}

/// A compile that fails is recorded too, since it is logged before it runs.
#[test]
fn compile_commands_include_a_failed_compile() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    let result = gnu_build(&test, &collector)
        .file("foo.c")
        .env("CC_SHIM_FAIL_IF_ARG", "-c")
        .cargo_warnings(false)
        .try_compile("foo");
    assert!(result.is_err());

    let commands: Vec<_> = collector.commands().collect();
    assert_eq!(commands.len(), 1, "{commands:#?}");
    assert_eq!(arguments(&commands[0]), recorded_command(&test, "foo.c"));
}

/// Several `Build`s, their clones and repeated compiles all add to one
/// collector, in the order the compiles start.
#[test]
fn compile_command_collector_is_shared() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    let mut first = gnu_build(&test, &collector);
    first.file("foo.c");
    first.try_compile("foo").unwrap();
    // Reading the commands in between is fine.
    assert_eq!(collector.commands().len(), 1);
    first.try_compile("foo").unwrap();
    gnu_build(&test, &collector)
        .file("bar.c")
        .try_compile("bar")
        .unwrap();
    first
        .clone()
        .file("baz.c")
        .try_compile_intermediates()
        .unwrap();
    first
        .clone()
        .message_logger(None)
        .file("qux.c")
        .try_compile("qux")
        .unwrap();

    let cwd = env::current_dir().unwrap();
    let commands: Vec<_> = collector.commands().collect();
    let files: Vec<_> = commands.iter().map(CompileCommand::file).collect();
    assert_eq!(
        files,
        ["foo.c", "foo.c", "bar.c", "foo.c", "baz.c"].map(|src| cwd.join(src)),
        "{commands:#?}"
    );
    assert_eq!(commands[0], commands[1]);
}

/// `commands` yields the commands collected when it was called, from either
/// end, and doesn't keep the collector locked while it is alive.
#[test]
fn compile_command_collector_commands_iterator() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    gnu_build(&test, &collector)
        .file("foo.c")
        .try_compile("foo")
        .unwrap();
    gnu_build(&test, &collector)
        .file("bar.c")
        .try_compile("bar")
        .unwrap();

    let mut commands = collector.commands();
    assert_eq!(commands.len(), 2);

    // Another thread can log into the collector meanwhile, without waiting.
    let mut build = gnu_build(&test, &collector);
    build.file("baz.c");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(build.try_compile("baz")).unwrap());
    receiver
        .recv_timeout(Duration::from_secs(30))
        .expect("logging waited for the iterator")
        .unwrap();
    assert_eq!(collector.commands().len(), 3);

    let cwd = env::current_dir().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands.next_back().unwrap().file(), cwd.join("bar.c"));
    assert_eq!(commands.len(), 1);
    assert_eq!(commands.next().unwrap().file(), cwd.join("foo.c"));
    assert_eq!(commands.next(), None);

    let files: Vec<_> = collector
        .commands()
        .rev()
        .map(|command| command.file().to_path_buf())
        .collect();
    assert_eq!(files, ["baz.c", "bar.c", "foo.c"].map(|src| cwd.join(src)));
}

/// The JSON holds every collected command.
#[test]
fn json_compilation_database_holds_the_commands() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    gnu_build(&test, &collector)
        .file("foo.c")
        .file("bar.c")
        .define("GREETING", "\"hello\"")
        .try_compile("foo")
        .unwrap();
    let commands: Vec<_> = collector.commands().collect();
    assert_eq!(commands.len(), 2, "{commands:#?}");

    let json = json_compilation_database(collector.commands()).to_string();

    // Enough for the paths and arguments here, which have no control
    // characters.
    let quote = |s: &OsStr| {
        let s = s
            .to_str()
            .unwrap()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        format!("\"{s}\"")
    };
    assert!(json.starts_with("[\n  {\n"), "{json}");
    assert!(json.ends_with("\n  }\n]\n"), "{json}");
    assert!(json.contains(r#""-DGREETING=\"hello\"""#), "{json}");
    for command in &commands {
        let quoted: Vec<_> = command.arguments().map(quote).collect();
        for line in [
            format!("\"directory\": {},", quote(command.directory().as_os_str())),
            format!("\"file\": {},", quote(command.file().as_os_str())),
            format!("\"arguments\": [{}],", quoted.join(", ")),
            format!("\"output\": {}\n", quote(command.output().as_os_str())),
        ] {
            assert!(json.contains(&line), "{line} not in {json}");
        }
    }
}

/// The formatter writes the same text into any writer, from any iterator of
/// commands.
#[test]
fn json_compilation_database_formats_into_any_writer() {
    let test = Test::gnu();
    let collector = Arc::new(CompileCommandCollector::new());
    gnu_build(&test, &collector)
        .file("foo.c")
        .try_compile("foo")
        .unwrap();

    let json = json_compilation_database(collector.commands()).to_string();
    assert!(json.starts_with("[\n  {\n    \"directory\": "), "{json}");
    let mut text = String::from("// compile commands\n");
    write!(text, "{}", json_compilation_database(collector.commands())).unwrap();
    assert_eq!(text, format!("// compile commands\n{json}"));
    let mut bytes = Vec::new();
    write!(bytes, "{}", json_compilation_database(collector.commands())).unwrap();
    assert_eq!(bytes, json.as_bytes());

    let commands: Vec<_> = collector.commands().collect();
    assert_eq!(json_compilation_database(commands).to_string(), json);
}
