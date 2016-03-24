pub struct Service {
	handle: u32
}

#[derive(Ipc)]
impl Service {
	fn action(&self, f: u64) {
	}
	pub fn new(&self, a: u32, b: u32) -> u64 {
		Service { handle: 0 }
	}
}
