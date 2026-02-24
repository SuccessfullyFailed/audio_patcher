use cpal::{ Device as CpalDevice, Host as CpalHost, Stream as CpalStream, StreamConfig as CpalStreamConfig, SampleRate as CpalSampleRate, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _ } };
use std::{ error::Error, sync::{ Mutex, MutexGuard } };



fn main() {
}



pub struct AudioPatcher<const SAMPLE_RATE:u32> {
	input_devices:Vec<InputDevice<SAMPLE_RATE>>,
	effect_channels:Vec<EffectChannel>,
	output_devices:Vec<OutputDevice<SAMPLE_RATE>>
}
impl<const SAMPLE_RATE:u32> AudioPatcher<SAMPLE_RATE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new patcher.
	pub fn new(input_device_names:&[&str], output_device_names:&[&str]) -> Result<Self, Box<dyn Error>> {
		let mut patcher:AudioPatcher<SAMPLE_RATE> = AudioPatcher { input_devices: Vec::new(), effect_channels: Vec::new(), output_devices: Vec::new() };
		for device_name in input_device_names {
			patcher.add_input_device(device_name)?;
		}
		for device_name in output_device_names {
			patcher.add_output_device(device_name)?;
		}
		Ok(patcher)
	}

	/// Add a new input device.
	pub fn add_input_device(&mut self, device_name:&str) -> Result<(), Box<dyn Error>> {
		match InputDevice::new(device_name)? {
			Some(device) => {
				self.input_devices.push(device);
				Ok(())
			},
			None => Err(format!("Could not find input device with name '{device_name}'.").into())
		}
	}

	/// Add a new output device.
	pub fn add_output_device(&mut self, device_name:&str) -> Result<(), Box<dyn Error>> {
		match OutputDevice::new(device_name)? {
			Some(device) => {
				self.output_devices.push(device);
				Ok(())
			},
			None => Err(format!("Could not find input device with name '{device_name}'.").into())
		}
	}



	/* USAGE METHODS */

	/// Run the system, starting all streams.
	pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
		
		// Run streams.
		for input_device in &mut self.input_devices {
			input_device.create_stream()?;
		}
		for output_device in &mut self.output_devices {
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