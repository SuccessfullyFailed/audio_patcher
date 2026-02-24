use std::sync::{ Mutex, MutexGuard };



#[derive(PartialEq, Clone, Copy)]
pub enum DeviceType { Input, EffectChannel, Output }



#[derive(PartialEq, Clone, Copy)]
pub struct DeviceId {
	pub(crate) id:usize,
	pub(crate) device_type:DeviceType
}
impl DeviceId {
	pub fn new(device_type:DeviceType) -> DeviceId {
		static ID_GENERATOR:Mutex<usize> = Mutex::new(0);
		let mut generator_handle:MutexGuard<'_, usize> = ID_GENERATOR.lock().unwrap();
		*generator_handle += 1;
		DeviceId {
			id: *generator_handle - 1,
			device_type
		}
	}
}