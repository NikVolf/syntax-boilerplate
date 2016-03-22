extern crate syntex;
extern crate ethcore_ipc as codegen;

use std::env;
use std::path::Path;

pub fn main() {
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let src = Path::new("attrs.rs.in");
	let dst = Path::new(&out_dir).join("attrs_cg.rs");

	let mut registry = syntex::Registry::new();

	codegen::register(&mut registry);
	registry.expand("", &src, &dst).unwrap();
}
