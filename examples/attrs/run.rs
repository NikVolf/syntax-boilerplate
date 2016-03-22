extern crate ethcore_ipc as codegen;
mod attrs;

use codegen::interface::IpcInterface;

pub fn main() {
	let service = attrs::service::Service::new();
	service.call();
}
