use std::io::*;

pub trait IpcInterface<T> {
	/// reads the message from io, dispatches the call and returns result
	fn dispatch(&self, &mut Read) -> Vec<u8>;
	/// encodes the invocation, writes payload and waits for result
	fn invoke(&self, method_num: u16, &mut Write) -> Vec<u8>;
}
