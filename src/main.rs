use audio_buffer::AudioBuffer;
use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use crate::id_handling::{ DeviceId, DeviceType };
use circular_buffer::CircularBuffer;
use std::{error::Error, thread::sleep, time::Duration};



mod id_handling;



fn main() {
	let mut patcher:AudioPatcher<48000, 1024> = AudioPatcher::<48_000, 1024>::new(&["default"], &["default"], &[("default", "default")]).unwrap();
	patcher.run().unwrap();
}




enum ConnectionState { Idle, Parsing, Finished }

pub struct AudioPatcher<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	input_devices:Vec<InputDevice<SAMPLE_RATE, BUFFER_SIZE>>,
	effect_channels:Vec<EffectChannel>,
	output_devices:Vec<OutputDevice<SAMPLE_RATE, BUFFER_SIZE>>,
	connections:Vec<(ConnectionState, DeviceId, Vec<DeviceId>)> // Each output has a list of inputs to combine from.
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> AudioPatcher<SAMPLE_RATE, BUFFER_SIZE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new patcher.
	pub fn new(input_device_names:&[&str], output_device_names:&[&str], connections:&[(&str, &str)]) -> Result<Self, Box<dyn Error>> {

		// Create initial patcher.
		let mut patcher:AudioPatcher<SAMPLE_RATE, BUFFER_SIZE> = AudioPatcher {
			input_devices: Vec::new(),
			effect_channels: Vec::new(),
			output_devices: Vec::new(),
			connections: Vec::new()
		};

		// Add devices and connections.
		for device_name in input_device_names {
			patcher.add_input_device(device_name)?;
		}
		for device_name in output_device_names {
			patcher.add_output_device(device_name)?;
		}
		for (source_name, target_name) in connections {
			patcher.add_connection(source_name, target_name)?;
		}

		// Return patcher.
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

	/// Add a new link between two devices.
	pub fn add_connection(&mut self, source_device_name:&str, target_device_name:&str) -> Result<(), Box<dyn Error>> {

		// Find source device.
		let mut source_device_id:Option<DeviceId> = None;
		if let Some(input_device) = self.input_devices.iter().find(|device| device.name == source_device_name) {
			source_device_id = Some(input_device.id);
		} else if let Some(effect_device) = self.effect_channels.iter().find(|device| device.name == source_device_name) {
			source_device_id = Some(effect_device.id);
		}
		if source_device_id.is_none() {
			return Err(format!("Could not create link, no source device with name {source_device_name} was found.").into());
		}
		let source_device_id:DeviceId = source_device_id.unwrap();

		// Find target device.
		let mut target_device_id:Option<DeviceId> = None;
		if let Some(output_device) = self.output_devices.iter().find(|device| device.name == target_device_name) {
			target_device_id = Some(output_device.id);
		} else if let Some(effect_device) = self.effect_channels.iter().find(|device| device.name == target_device_name) {
			target_device_id = Some(effect_device.id);
		}
		if target_device_id.is_none() {
			return Err(format!("Could not create link, no target device with name {target_device_name} was found.").into());
		}
		let target_device_id:DeviceId = target_device_id.unwrap();

		// Create link and return success.
		let connections_for_target:&mut (ConnectionState, DeviceId, Vec<DeviceId>) = match self.connections.iter_mut().find(|(_, target_id, _)| target_id == &target_device_id) {
			Some(existing_connection_source) => existing_connection_source,
			None => {
				self.connections.push((ConnectionState::Idle, target_device_id, Vec::new()));
				self.connections.last_mut().unwrap()
			}
		};
		if !connections_for_target.2.contains(&source_device_id) {
			connections_for_target.2.push(source_device_id);
		}
		Ok(())
	}



	/* USAGE METHODS */

	/// Run the system, starting all streams. Will stream indefinitely.
	pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
		
		// Run streams.
		for input_device in &mut self.input_devices {
			input_device.create_stream()?;
		}
		for output_device in &mut self.output_devices {
			output_device.create_stream()?;
		}

		// Keep passing audio buffers through connections.
		const INTERVAL:Duration = Duration::from_millis(10);
		loop {
			for index in 0..self.connections.len() {
				self.handle_buffer_for_connection(index)?;
			}
			for connection in &mut self.connections {
				connection.0 = ConnectionState::Idle;
			}
			sleep(INTERVAL);
		}
	}

	fn handle_buffer_for_connection(&mut self, connection_index:usize) -> Result<(), Box<dyn Error>> {
		match self.connections[connection_index].0 {
			ConnectionState::Idle => {
				self.connections[connection_index].0 = ConnectionState::Parsing;

				// Handle buffers of connected devices first.
				for source_device_id in self.connections[connection_index].2.clone() {
					if let Some(sub_connections_index) = self.connections.iter().position(|connection| connection.1 == source_device_id) {
						self.handle_buffer_for_connection(sub_connections_index)?;
					}
				}

				// Get the size of buffer able to be taken from all connected sources.
				let mut combined_buffer_size:usize = usize::MAX;
				for source_device_id in &self.connections[connection_index].2 {
					match source_device_id.device_type {
						DeviceType::Input => {
							if let Some(device) = self.input_devices.iter().find(|device| &device.id == source_device_id) {
								println!("AVAILABLE: {}", device.buffer.currently_stored());
								combined_buffer_size = combined_buffer_size.min(device.buffer.currently_stored());
							}
						},
						DeviceType::EffectChannel => {},
						DeviceType::Output => {}
					}
				}

				// Create a buffer from all sources.
				if combined_buffer_size < usize::MAX {
					let mut source_buffers:Vec<Vec<f32>> = Vec::with_capacity(self.connections[connection_index].2.len());
					for source_device_id in &self.connections[connection_index].2 {
						match source_device_id.device_type {
							DeviceType::Input => {
								if let Some(device) = self.input_devices.iter_mut().find(|device| &device.id == source_device_id) {
									source_buffers.push(device.buffer.take(combined_buffer_size));
								}
							},
							DeviceType::EffectChannel => {},
							DeviceType::Output => {}
						}
					}
					if !source_buffers.is_empty() {
						let mut buffers:Vec<AudioBuffer> = source_buffers.into_iter().map(|buffer| AudioBuffer::new(buffer, 1, SAMPLE_RATE)).collect();
						let mut combined_buffer = buffers.remove(buffers.len() - 1);
						combined_buffer.combine_with(buffers);
						println!("COMBINED LEN: {}", combined_buffer.data().len());

						match self.connections[connection_index].1.device_type {
							DeviceType::Input => {},
							DeviceType::EffectChannel => {},
							DeviceType::Output => {
								if let Some(device) = self.output_devices.iter_mut().find(|device| device.id == self.connections[connection_index].1) {
									println!("WRITING");
									device.buffer.extend(combined_buffer.data());
								}
							}
						}
					}
				}

				self.connections[connection_index].0 = ConnectionState::Finished;
				Ok(())
			},
			ConnectionState::Parsing => {
				Err("Infinite loop in connections detected, removing problematic connection.".into())
			},
			ConnectionState::Finished => {
				Ok(())
			}
		}
	}
}



pub struct InputDevice<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	id:DeviceId,
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>,
	buffer:CircularBuffer<f32, BUFFER_SIZE>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> InputDevice<SAMPLE_RATE, BUFFER_SIZE> {

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
					id: DeviceId::new(DeviceType::Input),
					name: device_name.to_string(),
					device: cpal_device,
					stream: None,
					buffer: CircularBuffer::new()
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

		// Build stream.
		// As both the device and stream are stored in the same struct, the stream can only go on as long as the device and stream exist.
		// This means a raw pointer to the buffer is safe.
		let buffer_ptr:u64 = &mut self.buffer as *mut CircularBuffer<f32, BUFFER_SIZE> as u64;
		let stream:CpalStream = self.device.build_input_stream(
			&config,
			move |data:&[f32], _| {
				let mut buffer:CircularBuffer<f32, BUFFER_SIZE> = unsafe { *(buffer_ptr as *mut CircularBuffer<f32, BUFFER_SIZE>) };
				buffer.extend(data);
			},
			|err:CpalStreamError| eprintln!("{err}"),
			None
		)?;
		
		// Play stream and store in own properties.
		stream.play()?;
		self.stream = Some(stream);

		// Return success.
		Ok(())
	}
}



pub struct OutputDevice<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	id:DeviceId,
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>,
	buffer:CircularBuffer<f32, BUFFER_SIZE>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> OutputDevice<SAMPLE_RATE, BUFFER_SIZE> {

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
					id: DeviceId::new(DeviceType::Output),
					name: device_name.to_string(),
					device: cpal_device,
					stream: None,
					buffer: CircularBuffer::new()
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
		// As both the device and stream are stored in the same struct, the stream can only go on as long as the device and stream exist.
		// This means a raw pointer to the buffer is safe.
		let buffer_ptr:u64 = &mut self.buffer as *mut CircularBuffer<f32, BUFFER_SIZE> as u64;
		self.stream = Some(
			self.device.build_output_stream(
				&config,
				move |data:&mut [f32], _| {
					let mut buffer:CircularBuffer<f32, BUFFER_SIZE> = unsafe { *(buffer_ptr as *mut CircularBuffer<f32, BUFFER_SIZE>) };
					let target_size:usize = data.len();
					if buffer.currently_stored() > target_size {
						data.clone_from_slice(&buffer.take(target_size));
					}
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
	id:DeviceId,
	name:String
}
impl EffectChannel {

	/// Create a new, empty channel.
	pub fn new_empty() -> EffectChannel {
		EffectChannel {
			id: DeviceId::new(DeviceType::EffectChannel),
			name: String::new()
		}
	}
}