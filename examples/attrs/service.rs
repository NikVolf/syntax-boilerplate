pub struct Service {
	handle: u32
}

#[derive(Ipc)]
impl Service {
	fn action(f: u64) {
	}
	pub fn new(a: u32, b: u32) -> Self {
		Service { handle: 0 }
	}
}
