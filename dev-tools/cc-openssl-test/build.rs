use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    if cfg!(unix) && !target.contains("tvos") {
        openssl_src::Build::new().build();
    }
}
