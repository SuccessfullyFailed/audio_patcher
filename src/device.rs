use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use crate::{ PATCHER_INPUT_BUFFERS, PATCHER_OUTPUT_BUFFERS, SAMPLE_RATE, PatcherChannelId };
use circular_buffer::{ CircularBufferMultiRead, ReadCursor };
use std::error::Error;



pub struct InputDevice {
	device:CpalDevice,
	stream:Option<CpalStream>
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
					device: cpal_device,
					stream: None
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self, patcher_channel_id:&PatcherChannelId) -> Result<(), Box<dyn Error>> {

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
		
		// Play stream and return success.
		stream.play()?;
		self.stream = Some(stream);
		Ok(())
	}
}



pub struct OutputDevice {
	device:CpalDevice,
	stream:Option<CpalStream>
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
					device: cpal_device,
					stream: None
				}),
				None => None
			}
		)
	}



	/* USAGE METHODS */

	/// Create an input stream.
	pub fn create_stream(&mut self, patcher_channel_id:&PatcherChannelId, buffer_cursor:ReadCursor) -> Result<(), Box<dyn Error>> {

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
		
		// Play stream and return success.
		stream.play()?;
		self.stream = Some(stream);
		Ok(())
	}
}