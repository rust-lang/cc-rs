#![allow(missing_docs)]
#![cfg(feature = "compile-commands")]

use std::{
    any::Any,
    env,
    ffi::{OsStr, OsString},
    fmt::Write as _,
    io::Write as _,
    mem,
    panic::{RefUnwindSafe, UnwindSafe},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use cc::{
    compile_commands::{
        json_compilation_database, CompileCommand, CompileCommandCollector, CompileCommands,
    },
    BuildMessage, BuildMessageKind, BuildMessageLogger,
};

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

/// Runs `f` on another thread, so that reading commands that waits for more
/// fails the test instead of hanging it.
#[track_caller]
fn without_waiting<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(f()));
    receiver
        .recv_timeout(Duration::from_secs(30))
        .expect("reading the commands didn't finish")
}

/// The commands logged since the last read.
#[track_caller]
fn read(commands: &mut CompileCommands) -> Vec<CompileCommand> {
    // A stand-in while the commands are read on another thread.
    let mut taken = mem::replace(commands, CompileCommandCollector::new().1);
    let (read, taken) = without_waiting(move || (taken.by_ref().collect::<Vec<_>>(), taken));
    *commands = taken;
    read
}

#[test]
fn compile_commands_auto_traits() {
    fn assert_auto_traits<T: Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}
    assert_auto_traits::<CompileCommand>();
    assert_auto_traits::<CompileCommandCollector>();
    // The commands can be read on another thread.
    fn assert_send<T: Send + Unpin>() {}
    assert_send::<CompileCommands>();
}

fn collector() -> (Arc<CompileCommandCollector>, CompileCommands) {
    let (collector, commands) = CompileCommandCollector::new();
    (Arc::new(collector), commands)
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
    let (collector, mut commands) = collector();
    gnu_build(&test, &collector)
        .file("foo.c")
        .file("bar.c")
        .file("baz.S")
        .define("GREETING", "\"hello world\"")
        .try_compile("foo")
        .unwrap();

    let cwd = env::current_dir().unwrap();
    let commands = read(&mut commands);
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
    let (collector, mut commands) = collector();
    test.gcc()
        .file("foo.c")
        .file("bar.asm")
        .message_logger(Some(collector.clone()))
        .try_compile("foo")
        .unwrap();

    let commands = read(&mut commands);
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
    let (collector, mut commands) = collector();
    let result = gnu_build(&test, &collector)
        .file("foo.c")
        .env("CC_SHIM_FAIL_IF_ARG", "-c")
        .cargo_warnings(false)
        .try_compile("foo");
    assert!(result.is_err());

    let commands = read(&mut commands);
    assert_eq!(commands.len(), 1, "{commands:#?}");
    assert_eq!(arguments(&commands[0]), recorded_command(&test, "foo.c"));
}

/// Several `Build`s, their clones and repeated compiles all add to one
/// collector, in the order the compiles start.
#[test]
fn compile_command_collector_is_shared() {
    let test = Test::gnu();
    let (collector, mut commands) = collector();
    let mut first = gnu_build(&test, &collector);
    first.file("foo.c");
    first.try_compile("foo").unwrap();
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
    let commands = read(&mut commands);
    let files: Vec<_> = commands.iter().map(CompileCommand::file).collect();
    assert_eq!(
        files,
        ["foo.c", "foo.c", "bar.c", "foo.c", "baz.c"].map(|src| cwd.join(src)),
        "{commands:#?}"
    );
    assert_eq!(commands[0], commands[1]);
}

/// The commands can be read while the collector is still in use, also after
/// a `Build` on another thread logged into it. Each read yields the commands
/// logged since the last one, without waiting for more.
#[test]
fn compile_commands_yield_the_commands_logged_so_far() {
    let test = Test::gnu();
    let (collector, mut commands) = collector();
    let mut build = gnu_build(&test, &collector);
    build.file("foo.c");
    build.try_compile("foo").unwrap();
    gnu_build(&test, &collector)
        .file("bar.c")
        .try_compile("bar")
        .unwrap();

    let cwd = env::current_dir().unwrap();
    let files = |commands: Vec<CompileCommand>| -> Vec<_> {
        commands
            .iter()
            .map(|command| command.file().to_path_buf())
            .collect()
    };
    assert_eq!(
        files(read(&mut commands)),
        ["foo.c", "bar.c"].map(|src| cwd.join(src))
    );
    assert!(read(&mut commands).is_empty());

    let mut other = gnu_build(&test, &collector);
    other.file("baz.c");
    thread::spawn(move || other.try_compile("baz"))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(files(read(&mut commands)), [cwd.join("baz.c")]);

    // Commands logged before the collector is dropped can still be read.
    build.try_compile("foo").unwrap();
    drop((build, collector));
    assert_eq!(files(read(&mut commands)), [cwd.join("foo.c")]);
    assert!(read(&mut commands).is_empty());
}

/// Counts the compile commands it gets.
struct CompileCommandCounter(Arc<AtomicUsize>);

impl BuildMessageLogger for CompileCommandCounter {
    fn log(&self, kind: BuildMessageKind, _: BuildMessage<'_>, extra: &dyn Any) {
        if kind == BuildMessageKind::CompileCommand {
            assert!(extra.is::<CompileCommand>());
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// The collector passes the compile commands on to `forward_to` as well, and
/// keeps doing so once its `CompileCommands` is dropped.
#[test]
fn compile_command_collector_forwards_messages() {
    let test = Test::gnu();
    let count = Arc::new(AtomicUsize::new(0));
    let (collector, mut commands) = CompileCommandCollector::new();
    let collector = collector.forward_to(Box::new(CompileCommandCounter(count.clone())));
    let collector = Arc::new(collector);
    let mut build = gnu_build(&test, &collector);
    build.file("foo.c").file("bar.c");
    build.try_compile("foo").unwrap();
    assert_eq!(read(&mut commands).len(), 2);
    assert_eq!(count.load(Ordering::SeqCst), 2);

    drop(commands);
    build.try_compile("foo").unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 4);
}

/// The JSON holds every collected command.
#[test]
fn json_compilation_database_holds_the_commands() {
    let test = Test::gnu();
    let (collector, mut commands) = collector();
    gnu_build(&test, &collector)
        .file("foo.c")
        .file("bar.c")
        .define("GREETING", "\"hello\"")
        .try_compile("foo")
        .unwrap();
    let commands = read(&mut commands);
    assert_eq!(commands.len(), 2, "{commands:#?}");

    let json = json_compilation_database(commands.clone()).to_string();

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
/// commands, also straight from the `CompileCommands`.
#[test]
fn json_compilation_database_formats_into_any_writer() {
    let test = Test::gnu();
    let (collector, mut commands) = collector();
    let mut build = gnu_build(&test, &collector);
    build.file("foo.c");
    build.try_compile("foo").unwrap();
    let logged = read(&mut commands);

    let json = json_compilation_database(logged.clone()).to_string();
    assert!(json.starts_with("[\n  {\n    \"directory\": "), "{json}");
    let mut text = String::from("// compile commands\n");
    write!(text, "{}", json_compilation_database(logged.clone())).unwrap();
    assert_eq!(text, format!("// compile commands\n{json}"));
    let mut bytes = Vec::new();
    write!(
        bytes,
        "{}",
        json_compilation_database(logged.iter().cloned())
    )
    .unwrap();
    assert_eq!(bytes, json.as_bytes());

    // The same compile again, formatted while the collector is still alive.
    build.try_compile("foo").unwrap();
    let from_commands = without_waiting(move || json_compilation_database(commands).to_string());
    assert_eq!(from_commands, json);
}
