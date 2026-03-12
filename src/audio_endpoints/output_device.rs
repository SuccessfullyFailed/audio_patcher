use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use std::{ error::Error, sync::{ Arc, Mutex, MutexGuard } };
use circular_buffer::CircularBuffer;

use crate::audio_endpoint::AudioEndPoint;



pub struct OutputDevice<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>,
	buffer:Arc<Mutex<CircularBuffer<f32, BUFFER_SIZE>>>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> OutputDevice<SAMPLE_RATE, BUFFER_SIZE> {

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<Self>, Box<dyn Error>> {
		let host:CpalHost = cpal::default_host();
		let device_name_lowercase:String = device_name.to_lowercase();

		// Find cpal device.
		let cpal_device:Option<CpalDevice> = {
			if device_name_lowercase == "default" {
				host.default_output_device()
			} else {
				cpal::default_host().output_devices()?.into_iter().find(|device| device.name().is_ok_and(|name| name.to_lowercase().contains(&device_name_lowercase)))
			}
		};

		// Return new device.
		Ok(
			match cpal_device {
				Some(cpal_device) => Some(OutputDevice {
					name: device_name.to_string(),
					device: cpal_device,
					stream: None,
					buffer: Arc::new(Mutex::new(CircularBuffer::new()))
				}),
				None => None
			}
		)
	}
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> AudioEndPoint for OutputDevice<SAMPLE_RATE, BUFFER_SIZE> {
	

	/// Get the name of the output.
	fn name(&self) -> &str {
		&self.name
	}
	
	/// Start the output.
	fn start(&mut self) -> Result<(), Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_output_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let is_stereo:bool = config.channels == 2;
		let buffer_handle_ref:Arc<Mutex<CircularBuffer<f32, BUFFER_SIZE>>> = Arc::clone(&self.buffer);

		// Build stream and store in device.
		let stream:CpalStream = self.device.build_output_stream(
			&config,
			move |data:&mut [f32], _| {
				let mut buffer_handle:MutexGuard<'_, CircularBuffer<f32, BUFFER_SIZE>> = buffer_handle_ref.lock().unwrap();
				let take_amount:usize = data.len() * if is_stereo { 1 } else { 2 }; // For mono devices, take twice as much data and cut the right side off.

				// Take data and insert into device buffer.
				if buffer_handle.currently_stored() >= take_amount {
					data.copy_from_slice(&(
						if is_stereo {
							buffer_handle.take(take_amount)
						} else {
							buffer_handle.take(take_amount).chunks(2).map(|values| values[0]).collect()
						}
					));
				}

				// Skip excess buffer to prevent delays.
				while buffer_handle.currently_stored() > take_amount {
					buffer_handle.take(take_amount);
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

	/// Stop the output.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		self.stream = None;
		Ok(())
	}

	/// Wether or not this endpoint is currently using audio.
	/// Returns true when no audio is being used.
	fn is_idle(&self) -> bool {
		self.stream.is_none()
	}

	/// Pass additional data to the output.
	fn extend(&self, data:&[f32]) {
		self.buffer.lock().unwrap().extend(&data);
	}
}