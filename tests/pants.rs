extern crate gcc;

#[test]
fn test() {
	gcc::Config::new()
		.file("pants.c")
		.file("shirt.c")
		.compile("libpants.a");
}