use std::{sync::{ Mutex, MutexGuard }, usize};




#[derive(PartialEq, Clone)]
pub struct PatcherChannelId {
	pub(crate) index:usize,
	pub(crate) name:String
}
impl PatcherChannelId {
	pub fn new(index:usize, name:&str) -> PatcherChannelId {
		PatcherChannelId {
			index: index,
			name: name.to_string()
		}
	}
}




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