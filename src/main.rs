use cpal::{ Device as CpalDevice, Host as CpalHost, Stream as CpalStream, StreamConfig as CpalStreamConfig, SampleRate as CpalSampleRate, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _ } };
use std::{ error::Error, sync::{ Mutex, MutexGuard } };



fn main() {
}



pub struct AudioPatcher<const SAMPLE_RATE:u32, const MAX_INPUT_DEVICES:usize, const MAX_EFFECT_CHANNELS:usize, const MAX_OUTPUT_DEVICES:usize> {
	input_devices:[Option<InputDevice<SAMPLE_RATE>>; MAX_INPUT_DEVICES],
	effect_channels:[EffectChannel; MAX_EFFECT_CHANNELS],
	output_devices:[Option<OutputDevice<SAMPLE_RATE>>; MAX_OUTPUT_DEVICES]
}
impl<const SAMPLE_RATE:u32, const MAX_INPUT_DEVICES:usize, const MAX_EFFECT_CHANNELS:usize, const MAX_OUTPUT_DEVICES:usize> AudioPatcher<SAMPLE_RATE, MAX_INPUT_DEVICES, MAX_EFFECT_CHANNELS, MAX_OUTPUT_DEVICES> {

	/* CONSTRUCTOR METHODS */

	/// Create a new patcher.
	pub fn new(input_device_names:&[&str], output_device_names:&[&str]) -> Result<Self, Box<dyn Error>> {

		// Find input devices.
		let mut input_devices:[Option<InputDevice<SAMPLE_RATE>>; MAX_INPUT_DEVICES] = [const { None }; MAX_INPUT_DEVICES];
		for (index, name) in input_device_names.iter().enumerate() {
			match InputDevice::new(name)? {
				Some(device) => input_devices[index] = Some(device),
				None => eprintln!("Could not find input device by name '{name}'")
			}
		}

		// Find output devices.
		let mut output_devices:[Option<OutputDevice<SAMPLE_RATE>>; MAX_OUTPUT_DEVICES] = [const { None }; MAX_OUTPUT_DEVICES];
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



	/* USAGE METHODS */

	/// Run the system, starting all streams.
	pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
		
		// Run streams.
		for input_device in self.input_devices.iter_mut().flatten() {
			input_device.create_stream()?;
		}
		for output_device in self.output_devices.iter_mut().flatten() {
			output_device.create_stream()?;
		}

		// Return success.
		Ok(())
	}
}



static ID_GENERATOR:Mutex<usize> = Mutex::new(0);
fn generate_id() -> usize {
	let mut id_generator_handle:MutexGuard<'_, usize> = ID_GENERATOR.lock().unwrap();
	let new_id:usize = *id_generator_handle;
	*id_generator_handle += 1;
	new_id
}



pub struct InputDevice<const SAMPLE_RATE:u32> {
	id:usize,
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>
}
impl<const SAMPLE_RATE:u32> InputDevice<SAMPLE_RATE> {

	/* CONSTRUCTOR METHODS */

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<InputDevice<SAMPLE_RATE>>, Box<dyn Error>> {
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
					device: cpal_device,
					stream: None
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self) -> Result<(), Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_input_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let _channel_count:usize = config.channels as usize;

		// Build stream and store in device.
		self.stream = Some(
			self.device.build_input_stream(
				&config,
				move |data:&[f32], _| {
					
				},
				|err:CpalStreamError| eprintln!("{err}"),
				None
			)?
		);

		// Return success.
		Ok(())
	}
}



pub struct OutputDevice<const SAMPLE_RATE:u32> {
	id:usize,
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>
}
impl<const SAMPLE_RATE:u32> OutputDevice<SAMPLE_RATE> {

	/* CONSTRUCTOR METHODS */

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<OutputDevice<SAMPLE_RATE>>, Box<dyn Error>> {
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
					device: cpal_device,
					stream: None
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self) -> Result<(), Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_output_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let _channel_count:usize = config.channels as usize;

		// Build stream and store in device.
		self.stream = Some(
			self.device.build_output_stream(
				&config,
				move |data:&mut [f32], _| {
					
				},
				|err:CpalStreamError| eprintln!("{err}"),
				None
			)?
		);

		// Return success.
		Ok(())
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