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
}
