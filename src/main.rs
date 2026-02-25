use audio_buffer::AudioBuffer;
use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use crate::{ id_handling::{ InputDeviceId, OutputDeviceId }, settings::read_settings };
use std::{ error::Error, thread::sleep, time::Duration, usize };
use circular_buffer::CircularBuffer;
use mini_ini_parser::Ini;



mod id_handling;
mod settings;



const SAMPLE_RATE:u32 = 48_000;
const BUFFER_SIZE:usize = SAMPLE_RATE as usize;

const MAX_INPUT_DEVICES:usize = 8;
static mut INPUT_DEVICE_STEREO:[bool; MAX_INPUT_DEVICES] = [false; MAX_INPUT_DEVICES];
static mut INPUT_BUFFERS:[CircularBuffer<f32, BUFFER_SIZE>; MAX_INPUT_DEVICES] = [CircularBuffer::new_const(0.0); MAX_INPUT_DEVICES];

const MAX_OUTPUT_DEVICES:usize = 8;
static mut OUTPUT_DEVICE_STEREO:[bool; MAX_OUTPUT_DEVICES] = [false; MAX_OUTPUT_DEVICES];
static mut OUTPUT_BUFFERS:[CircularBuffer<f32, BUFFER_SIZE>; MAX_OUTPUT_DEVICES] = [CircularBuffer::new_const(0.0); MAX_OUTPUT_DEVICES];

const MAX_CONNECTIONS_PER_NODE:usize = 8;
static mut CONNECTIONS:[Connection; MAX_OUTPUT_DEVICES] = [Connection::new_const(); MAX_OUTPUT_DEVICES];



fn main() -> Result<(), Box<dyn Error>> {

	// Read settings.
	let settings:Ini = read_settings()?;
	let input_device_names:Vec<&str> = settings["devices"]["input"].value.split(",").map(|word| word.trim()).collect();
	let output_device_names:Vec<&str> = settings["devices"]["output"].value.split(",").map(|word| word.trim()).collect();
	let mut connection_sources:Vec<(&str, Vec<&str>)> = Vec::new();
	for connection_source in settings["devices"]["connections"].value.split(",") {
		let split:Vec<&str> = connection_source.split("->").collect();
		let input_device_name:&str = split[0].trim();
		let output_device_names:Vec<&str> = split[1].split(",").map(|name| name.trim()).collect();
		connection_sources.push((input_device_name, output_device_names));
	}

	// Find devices.
	let mut input_devices:Vec<InputDevice> = Vec::new();
	let mut output_devices:Vec<OutputDevice> = Vec::new();
	for device_name in input_device_names {
		if let Some(device) = InputDevice::new(device_name)? {
			input_devices.push(device);
		}
	}
	for device_name in output_device_names {
		if let Some(device) = OutputDevice::new(device_name)? {
			output_devices.push(device);
		}
	}

	// Build connections.
	for (input_device_name, output_device_names) in connection_sources {
		if let Some(input_device_index) = input_devices.iter().position(|device| device.name == input_device_name) {
			for output_device_name in output_device_names {
				if let Some(output_device) = output_devices.iter().find(|device| device.name == output_device_name) {
					unsafe { CONNECTIONS[input_device_index].add_connection(output_device.id); }
				}
			}
		}
	}

	// Start streams for devices.
	let mut streams:Vec<CpalStream> = Vec::with_capacity(input_devices.len() + output_devices.len());
	for device in &mut input_devices {
		streams.push(device.create_stream()?);
	}
	for device in &mut output_devices {
		streams.push(device.create_stream()?);
	}

	// Keep moving data from buffers to their targets.
	const INTERVAL:Duration = Duration::from_millis(10);
	loop {
		for input_device_index in 0..input_devices.len() {
			handle_connection_for_input_device_index(input_device_index)?;
		}
		for input_device_index in 0..input_devices.len() {
			unsafe { CONNECTIONS[input_device_index].status = 0; }
		}
		sleep(INTERVAL);
	}
}


fn handle_connection_for_input_device_index(input_device_index:usize) -> Result<(), Box<dyn Error>> {
	unsafe {
		let connection:&mut Connection = &mut CONNECTIONS[input_device_index];
		match connection.status {
			0 => {
				connection.status = 1;

				// Take buffer from input.
				let input_is_stereo:bool = INPUT_DEVICE_STEREO[input_device_index];
				let audio_data:Vec<f32> = INPUT_BUFFERS[input_device_index].take_all();

				// Move data to outputs.
				let mut stereo_converted_data:Option<AudioBuffer> = None;
				for output_id in CONNECTIONS[input_device_index].connected_outputs() {
					let output_is_stereo:bool = OUTPUT_DEVICE_STEREO[output_id.index];
					if input_is_stereo == output_is_stereo {
						OUTPUT_BUFFERS[output_id.index].extend(&audio_data);
					} else {
						if stereo_converted_data.is_none() {
							let mut converted_data:AudioBuffer = AudioBuffer::new(audio_data.clone(), if input_is_stereo { 2 } else { 1 }, SAMPLE_RATE);
							converted_data.resample(if output_is_stereo { 2 } else { 1 }, SAMPLE_RATE);
							stereo_converted_data = Some(converted_data);
						}
						OUTPUT_BUFFERS[output_id.index].extend(stereo_converted_data.as_ref().unwrap().data());
					}
				}

				// Return success.
				connection.status = 2;
				Ok(())
			},
			1 => {
				Err("Infinite loop found in connections.".into())
			},
			_ => {
				Ok(())
			}
		}
	}
}






pub struct InputDevice {
	id:InputDeviceId,
	name:String,
	device:CpalDevice
}
impl InputDevice {

	/* CONSTRUCTOR METHODS */

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<Self>, Box<dyn Error>> {
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
					id: InputDeviceId::new(),
					name: device_name.to_string(),
					device: cpal_device
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self) -> Result<CpalStream, Box<dyn Error>> {
		let device_index:usize = self.id.index;

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_input_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		if config.channels == 2 {
			unsafe { INPUT_DEVICE_STEREO[device_index] = true; }
		}

		// Build stream.
		let device_index:usize = self.id.index;
		let stream:CpalStream = self.device.build_input_stream(
			&config,
			move |data:&[f32], _| unsafe {
				INPUT_BUFFERS[device_index].extend(data);
			},
			|err:CpalStreamError| eprintln!("{err}"),
			None
		)?;
		
		// Play and return stream.
		stream.play()?;
		Ok(stream)
	}
}



pub struct OutputDevice {
	id:OutputDeviceId,
	name:String,
	device:CpalDevice
}
impl OutputDevice {

	/* CONSTRUCTOR METHODS */

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<Self>, Box<dyn Error>> {
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
					id: OutputDeviceId::new(),
					name: device_name.to_string(),
					device: cpal_device
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self) -> Result<CpalStream, Box<dyn Error>> {
		let device_index:usize = self.id.index;

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_output_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		if config.channels == 2 {
			unsafe { OUTPUT_DEVICE_STEREO[device_index] = true; }
		}

		// Build stream and store in device.
		let stream = self.device.build_output_stream(
			&config,
			move |data:&mut [f32], _| unsafe {
				let target_size:usize = data.len();
				if OUTPUT_BUFFERS[device_index].currently_stored() > target_size {
					data.clone_from_slice(&OUTPUT_BUFFERS[device_index].take(target_size));
				}
			},
			|err:CpalStreamError| eprintln!("{err}"),
			None
		)?;
		
		// Play and return stream.
		stream.play()?;
		Ok(stream)
	}
}


#[derive(PartialEq, Clone, Copy)]
pub struct Connection {
	target_devices:[Option<OutputDeviceId>; MAX_CONNECTIONS_PER_NODE],
	target_device_cursor:usize,
	status:u8
}
impl Connection {

	/* CONSTRUCTOR METHODS */

	/// Create a new, empty connection.
	pub const fn new_const() -> Connection {
		Connection {
			target_devices: [const { None }; MAX_CONNECTIONS_PER_NODE],
			target_device_cursor: 0,
			status: 0
		}
	}



	/* USAGE METHODS */

	/// Add a new output connection.
	pub fn add_connection(&mut self, target_output:OutputDeviceId) {
		if self.target_device_cursor < MAX_CONNECTIONS_PER_NODE {
			self.target_devices[self.target_device_cursor] = Some(target_output);
			self.target_device_cursor += 1;
		} else {
			eprintln!("Connection has reached max target devices count.");
		}
	}

	/// Get all connected output device indexes.
	pub fn connected_outputs(&self) -> Vec<OutputDeviceId> {
		self.target_devices.iter().flatten().copied().collect::<Vec<OutputDeviceId>>()
	}
}