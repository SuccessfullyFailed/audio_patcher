use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use crate::{ audio_effect::AudioEffect, id_handling::{ InputDeviceId, OutputDeviceId, PatcherChannelId }, settings::read_settings };
use circular_buffer::{ CircularBuffer, CircularBufferMultiRead, ReadCursor };
use std::{ error::Error, thread::sleep, time::{Duration, Instant}, usize };
use mini_ini_parser::Ini;



mod id_handling;
mod settings;
mod audio_effect;
mod audio_effects;



const SAMPLE_RATE:u32 = 48_000;
const BUFFER_SIZE:usize = SAMPLE_RATE as usize;
const BATCHES_PER_SECOND:u32 = 100;
const BATCH_SIZE:usize = SAMPLE_RATE as usize / BATCHES_PER_SECOND as usize;

const MAX_PATCHER_CHANNELS:usize = 32;
static mut PATCHER_CHANNELS:[Option<PatcherChannel>; MAX_PATCHER_CHANNELS] = [const { None }; MAX_PATCHER_CHANNELS];
static mut PATCHER_INPUT_BUFFERS:[CircularBuffer<f32, BUFFER_SIZE>; MAX_PATCHER_CHANNELS] = [CircularBuffer::new_const(0.0); MAX_PATCHER_CHANNELS];
static mut PATCHER_OUTPUT_BUFFERS:[CircularBufferMultiRead<f32, BUFFER_SIZE, MAX_PATCHER_CHANNELS>; MAX_PATCHER_CHANNELS] = [CircularBufferMultiRead::new_const(0.0); MAX_PATCHER_CHANNELS];



fn main() -> Result<(), Box<dyn Error>> {

	// Read settings.
	let settings:Ini = read_settings()?;
	let input_device_names:Vec<&str> = settings["devices"]["input"].value.split(",").map(|word| word.trim()).collect();
	let output_device_names:Vec<&str> = settings["devices"]["output"].value.split(",").map(|word| word.trim()).collect();
	let mut connection_sources:Vec<(&str, Vec<&str>)> = Vec::new();
	for connection_source in settings["devices"]["connections"].value.split(",") {
		let split:Vec<&str> = connection_source.split("->").collect();
		let target_channel_name:&str = split[0].trim();
		let connection_channel_names:Vec<&str> = split[1].split(",").map(|name| name.trim()).collect();
		connection_sources.push((target_channel_name, connection_channel_names));
	}

	// Find devices.
	let mut input_devices:Vec<InputDevice> = Vec::new();
	let mut output_devices:Vec<OutputDevice> = Vec::new();
	for device_name in input_device_names {
		if let Some(device) = InputDevice::new(device_name)? {
			input_devices.push(device);
		} else {
			eprintln!("Could not find input device by name '{device_name}'.");
		}
	}
	for device_name in output_device_names {
		if let Some(device) = OutputDevice::new(device_name)? {
			output_devices.push(device);
		} else {
			eprintln!("Could not find output device by name '{device_name}'.");
		}
	}

	// Build patcher channels.
	for (input_device_index, input_device) in input_devices.into_iter().enumerate() {
		let channel_index:usize = MAX_PATCHER_CHANNELS - input_device_index - 1;
		unsafe {
			if PATCHER_CHANNELS[channel_index].is_none() {
				let mut channel:PatcherChannel = PatcherChannel::new(channel_index, &input_device.name);
				channel.input_device = Some(input_device);
				PATCHER_CHANNELS[channel_index] = Some(channel);
			} else {
				eprintln!("Could not create patcher channel for output device at index {channel_index} as the channel is already being used.");
			}
		}
	}
	for (output_device_index, output_device) in output_devices.into_iter().enumerate() {
		let channel_index:usize = output_device_index;
		unsafe {
			if PATCHER_CHANNELS[channel_index].is_none() {
				let mut channel:PatcherChannel = PatcherChannel::new(channel_index, &output_device.name);
				channel.output_device = Some(output_device);
				PATCHER_CHANNELS[channel_index] = Some(channel);
			} else {
				eprintln!("Could not create patcher channel for input device at index {channel_index} as the channel is already being used.");
			}
		}
	}

	// Build connections.
	// TODO: Clean up.
	for (target_channel_name, connection_channel_names) in connection_sources {
		#[allow(static_mut_refs)]
		unsafe {
			if let Some(target_channel_index) = PATCHER_CHANNELS.iter().position(|channel| channel.as_ref().is_some_and(|channel| channel.id.name == target_channel_name)) {
				for connection_channel_name in connection_channel_names {
					let mut valid_connection:bool = false;
					if let Some(connection_channel_index) = PATCHER_CHANNELS.iter().skip(target_channel_index + 1).position(|channel| channel.as_ref().is_some_and(|channel| channel.id.name == connection_channel_name)).map(|offset| target_channel_index + 1 + offset) {
						let connection_id:PatcherChannelId = PATCHER_CHANNELS[connection_channel_index].as_ref().unwrap().id.clone();
						if let Err(error) = PATCHER_CHANNELS[target_channel_index].as_mut().unwrap().connect_to_channel(&connection_id) {
							eprintln!("Could not create connection: {error}");
						} else {
							valid_connection = true;
						}
					}
					if !valid_connection {
						eprintln!("Could not create connection from '{target_channel_name}' to '{connection_channel_name}.");
					}
				}
			} else {
				eprintln!("Could not create connections from '{target_channel_name}' as the channel seems to not exist.");
			}
		}
	}

	// Create streams for devices from right to left.
	let mut streams:Vec<CpalStream> = Vec::new();
	for patcher_channel_index in (0..MAX_PATCHER_CHANNELS).rev() {
		unsafe {
			if let Some(channel) = &mut PATCHER_CHANNELS[patcher_channel_index] {
				if let Some(input_device) = &mut channel.input_device {
					streams.push(input_device.create_stream(&channel.id)?);
				}
				if let Some(output_device) = &mut channel.output_device {
					streams.push(output_device.create_stream(&channel.id, PATCHER_OUTPUT_BUFFERS[channel.id.index].create_read_cursor())?);
				}
			}
		}
	}

	// Start all streams.
	for stream in &streams {
		stream.play()?;
	}

	// Keep moving data from buffers to their targets.
	const INTERVAL_DELAY:Duration = Duration::from_millis(1);
	let mut last_interval:Instant = Instant::now() - INTERVAL_DELAY;
	loop {
		// Adhere to interval.
		let now:Instant = Instant::now();
		let duration_since_last_interval:Duration = now.duration_since(last_interval);
		if duration_since_last_interval < INTERVAL_DELAY {
			sleep(INTERVAL_DELAY - duration_since_last_interval);
		}
		last_interval = now;

		// Update buffers from right to left.
		for patcher_channel_index in (0..MAX_PATCHER_CHANNELS).rev() {
			unsafe {
				if let Some(channel) = &mut PATCHER_CHANNELS[patcher_channel_index] {
					let input_buffer:Vec<f32> = channel.get_input_buffer();
					PATCHER_OUTPUT_BUFFERS[channel.id.index].extend(&input_buffer);
				}
			}
		}
	}
}



pub struct PatcherChannel {
	id:PatcherChannelId,
	connections:Vec<(PatcherChannelId, ReadCursor)>,
	input_device:Option<InputDevice>,
	effects:Vec<Box<dyn AudioEffect>>,
	output_device:Option<OutputDevice>
}
impl PatcherChannel {

	/// Create a new channel.
	pub fn new(channel_index:usize, channel_name:&str) -> PatcherChannel {
		PatcherChannel {
			id: PatcherChannelId::new(channel_index, channel_name),
			connections: Vec::new(),
			input_device: None,
			effects: Vec::new(),
			output_device: None
		}
	}

	/// Set an input device.
	pub fn set_input_device(&mut self, input_device:InputDevice) {
		self.input_device = Some(input_device);
	}

	/// Set an output device.
	pub fn set_output_device(&mut self, output_device:OutputDevice) {
		self.output_device = Some(output_device);
	}

	/// Add a connection to another channel.
	pub(crate) fn connect_to_channel(&mut self, channel_id:&PatcherChannelId) -> Result<(), Box<dyn Error>> {
		if channel_id.index < self.id.index {
			Err(format!("Cannot create connection from channel {} to channel {}, can only create connections with higher indexes.", self.id.index, channel_id.index).into())
		} else {
			self.connections.push((channel_id.clone(), unsafe { PATCHER_OUTPUT_BUFFERS[channel_id.index].create_read_cursor() }));
			Ok(())
		}
	}

	/// Create a buffer from this channels' input device and connections combined.
	fn get_input_buffer(&mut self) -> Vec<f32> {

		// Get buffer from input device and connected channels.
		let input_device_buffer:Option<Vec<f32>> = unsafe {
			let source_buffer:&mut CircularBuffer<f32, 48000> = &mut PATCHER_INPUT_BUFFERS[self.id.index];
			if source_buffer.currently_stored() > BATCH_SIZE {
				Some(source_buffer.take(BATCH_SIZE))
			} else {
				None
			}
		};
		let mut connection_buffers:Vec<Vec<f32>> = Vec::new();
		for (connection, buffer_cursor) in &self.connections {
			unsafe {
				let buffer = &mut PATCHER_OUTPUT_BUFFERS[connection.index];
				if buffer.currently_stored(buffer_cursor) >= BATCH_SIZE {
					connection_buffers.push(buffer.take(BATCH_SIZE, buffer_cursor));
				}
			}
		}

		// Return combined buffers.
		if connection_buffers.is_empty() {
			input_device_buffer.unwrap_or_default()
		} else {
			let mut combined_buffer:Vec<f32> = input_device_buffer.unwrap_or(connection_buffers.remove(connection_buffers.len() - 1));
			let longest_buffer_len:usize = combined_buffer.len().max(connection_buffers.iter().map(|buffer| buffer.len()).max().unwrap_or_default());
			combined_buffer.extend(vec![0.0; longest_buffer_len - combined_buffer.len()]);

			// Looping through sample per index, then buffer index feels logical, but looping through entire buffers is more favorable with CPU cache.
			for additional_buffer in connection_buffers {
				for index in 0..additional_buffer.len() {
					combined_buffer[index] += additional_buffer[index];
				}
			}

			// Normalize sample, make sure the buffers don't exceed max volume.
			for sample in &mut combined_buffer {
				if *sample > 1.0 {
					*sample = 1.0;
				} else if *sample < -1.0 {
					*sample = -1.0;
				}
			}

			combined_buffer
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
	fn create_stream(&mut self, patcher_channel_id:&PatcherChannelId) -> Result<CpalStream, Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_input_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let is_stereo:bool = config.channels == 2;
		let buffer_index:usize = patcher_channel_id.index;

		// Build stream.
		let stream:CpalStream = self.device.build_input_stream(
			&config,
			move |data:&[f32], _| unsafe {
				if is_stereo {
					PATCHER_INPUT_BUFFERS[buffer_index].extend(data);
				} else {
					let stereo_data:Vec<f32> = data.into_iter().map(|value| [*value; 2]).flatten().collect();
					PATCHER_INPUT_BUFFERS[buffer_index].extend(&stereo_data);
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
	fn create_stream(&mut self, patcher_channel_id:&PatcherChannelId, buffer_cursor:ReadCursor) -> Result<CpalStream, Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_output_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let is_stereo:bool = config.channels == 2;
		let buffer_index:usize = patcher_channel_id.index;

		// Build stream and store in device.
		let stream:CpalStream = self.device.build_output_stream(
			&config,
			move |data:&mut [f32], _| unsafe {
				let data_len:usize = data.len();
				let buffer:&mut CircularBufferMultiRead<f32, 48000, 32> = &mut PATCHER_OUTPUT_BUFFERS[buffer_index];

				if is_stereo {
					let take_amount:usize = data_len;
					if buffer.currently_stored(&buffer_cursor) > take_amount {
						let buffer_data:Vec<f32> = buffer.take(take_amount, &buffer_cursor);
						if buffer_data.len() == take_amount {
							data.copy_from_slice(&buffer_data);
						}
					}
				}

				else {
					let take_amount:usize = data_len * 2;
					if buffer.currently_stored(&buffer_cursor) > take_amount {
						let buffer_data:Vec<f32> = buffer.take(take_amount, &buffer_cursor).chunks(2).map(|values| values[0]).collect();
						if buffer_data.len() == take_amount {
							data.copy_from_slice(&buffer_data);
						}
					}
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