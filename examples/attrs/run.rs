extern crate ethcore_ipc as codegen;
extern crate bincode;
extern crate serde;

mod attrs;

use codegen::interface::IpcInterface;

pub fn main() {
	let service = attrs::service::Service { handle: 0 };
}
