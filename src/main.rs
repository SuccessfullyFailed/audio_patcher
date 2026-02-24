use cpal::{ Device as CpalDevice, Host as CpalHost, traits::{DeviceTrait, HostTrait as _} };
use std::{error::Error, sync::{Mutex, MutexGuard}};



fn main() {
}



pub struct AudioPatcher<const MAX_INPUT_DEVICES:usize, const MAX_EFFECT_CHANNELS:usize, const MAX_OUTPUT_DEVICES:usize> {
	input_devices:[Option<InputDevice>; MAX_INPUT_DEVICES],
	effect_channels:[EffectChannel; MAX_EFFECT_CHANNELS],
	output_devices:[Option<OutputDevice>; MAX_OUTPUT_DEVICES]
}
impl<const MAX_INPUT_DEVICES:usize, const MAX_EFFECT_CHANNELS:usize, const MAX_OUTPUT_DEVICES:usize> AudioPatcher<MAX_INPUT_DEVICES, MAX_EFFECT_CHANNELS, MAX_OUTPUT_DEVICES> {

	/// Create a new patcher.
	pub fn new(input_device_names:&[&str], output_device_names:&[&str]) -> Result<Self, Box<dyn Error>> {

		// Find input devices.
		let mut input_devices:[Option<InputDevice>; MAX_INPUT_DEVICES] = [const { None }; MAX_INPUT_DEVICES];
		for (index, name) in input_device_names.iter().enumerate() {
			match InputDevice::new(name)? {
				Some(device) => input_devices[index] = Some(device),
				None => eprintln!("Could not find input device by name '{name}'")
			}
		}

		// Find output devices.
		let mut output_devices:[Option<OutputDevice>; MAX_OUTPUT_DEVICES] = [const { None }; MAX_OUTPUT_DEVICES];
		for (index, name) in output_device_names.iter().enumerate() {
			match OutputDevice::new(name)? {
				Some(device) => output_devices[index] = Some(device),
				None => eprintln!("Could not find input device by name '{name}'")
			}
		}

		// Create effect channels.
		let effect_channels:[EffectChannel; MAX_EFFECT_CHANNELS] = [const { EffectChannel::new_empty() }; MAX_EFFECT_CHANNELS];

		// Return full patcher.
		Ok(AudioPatcher {
			input_devices,
			effect_channels,
			output_devices
		})
	}
}



static ID_GENERATOR:Mutex<usize> = Mutex::new(0);
fn generate_id() -> usize {
	let mut id_generator_handle:MutexGuard<'_, usize> = ID_GENERATOR.lock().unwrap();
	let new_id:usize = *id_generator_handle;
	*id_generator_handle += 1;
	new_id
}



pub struct InputDevice {
	id:usize,
	name:String,
	device:CpalDevice
}
impl InputDevice {

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<InputDevice>, Box<dyn Error>> {
		let host:CpalHost = cpal::default_host();

		// Find cpal device.
		let cpal_device:Option<CpalDevice> = {
			if device_name.to_lowercase() == "default" {
				host.default_input_device()
			} else {
				cpal::default_host().input_devices()?.into_iter().find(|device| device.name().is_ok_and(|name| name == device_name))
			}
		};

		// Return new device.
		Ok(
			match cpal_device {
				Some(cpal_device) => Some(InputDevice {
					id: generate_id(),
					name: device_name.to_string(),
					device: cpal_device
				}),
				None => None
			}
		)
	}
}



pub struct OutputDevice {
	id:usize,
	name:String,
	device:CpalDevice
}
impl OutputDevice {

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<OutputDevice>, Box<dyn Error>> {
		let host:CpalHost = cpal::default_host();

		// Find cpal device.
		let cpal_device:Option<CpalDevice> = {
			if device_name.to_lowercase() == "default" {
				host.default_output_device()
			} else {
				cpal::default_host().output_devices()?.into_iter().find(|device| device.name().is_ok_and(|name| name == device_name))
			}
		};

		// Return new device.
		Ok(
			match cpal_device {
				Some(cpal_device) => Some(OutputDevice {
					id: generate_id(),
					name: device_name.to_string(),
					device: cpal_device
				}),
				None => None
			}
		)
	}
}



pub struct EffectChannel {
	id:usize,
	name:String
}
impl EffectChannel {

	/// Create a new, empty channel.
	pub const fn new_empty() -> EffectChannel {
		EffectChannel {
			id: 0,
			name: String::new()
		}
	}
}