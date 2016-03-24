pub trait IpcInterface<T> {
	/// reads the message from io, dispatches the call and returns result
	fn dispatch<R>(&self, r: &mut R) -> Vec<u8> where R: ::std::io::Read;
	/// encodes the invocation, writes payload and waits for result
	fn invoke<W>(&self, method_num: u16, params: &Option<Vec<u8>>, w: &mut W) -> Vec<u8> where W: ::std::io::Write;
}
