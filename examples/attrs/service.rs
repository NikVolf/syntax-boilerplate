pub struct Service {
	handle: u32
}

#[derive(Ipc)]
impl Service {
	fn action() {
	}
	pub fn new() -> Self {
		Service { handle: 0 }
	}
}
