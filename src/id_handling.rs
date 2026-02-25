use std::sync::{ Mutex, MutexGuard };




#[derive(PartialEq, Clone, Copy)]
pub struct InputDeviceId {
	pub(crate) index:usize
}
impl InputDeviceId {
	pub fn new() -> InputDeviceId {
		static ID_GENERATOR:Mutex<usize> = Mutex::new(0);
		let mut generator_handle:MutexGuard<'_, usize> = ID_GENERATOR.lock().unwrap();
		*generator_handle += 1;
		InputDeviceId {
			index: *generator_handle - 1
		}
	}
}



#[derive(PartialEq, Clone, Copy)]
pub struct OutputDeviceId {
	pub(crate) index:usize
}
impl OutputDeviceId {
	pub fn new() -> OutputDeviceId {
		static ID_GENERATOR:Mutex<usize> = Mutex::new(0);
		let mut generator_handle:MutexGuard<'_, usize> = ID_GENERATOR.lock().unwrap();
		*generator_handle += 1;
		OutputDeviceId {
			index: *generator_handle - 1
		}
	}
}