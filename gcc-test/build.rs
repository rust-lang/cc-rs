extern crate gcc;

fn main() {
    gcc::Config::new()
                .file("src/foo.c")
                .compile("libfoo.a");

    gcc::Config::new()
                .file("src/bar1.c")
                .file("src/bar2.c")
                .compile("libbar.a");

    gcc::Config::new()
                .file("src/bar1.c")
                .file("src/bar2.c")
                .compile("libbar.a");

    let target = std::env::var("TARGET").unwrap();
    let file = target.split("-").next().unwrap();
    let file = format!("src/{}.{}", file,
                       if target.contains("msvc") {"asm"} else {"S"});
    gcc::Config::new()
                .file(file)
                .compile("libasm.a");
}
