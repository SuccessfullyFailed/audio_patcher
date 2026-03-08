use cpal::{ Device as CpalDevice, Host as CpalHost, SampleRate as CpalSampleRate, Stream as CpalStream, StreamConfig as CpalStreamConfig, StreamError as CpalStreamError, traits::{ DeviceTrait, HostTrait as _, StreamTrait } };
use std::{ error::Error, sync::{ Arc, Mutex, MutexGuard } };
use circular_buffer::CircularBuffer;

use crate::audio_generator::AudioGenerator;



pub struct InputDevice<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	name:String,
	device:CpalDevice,
	stream:Option<CpalStream>,
	buffer:Arc<Mutex<CircularBuffer<f32, BUFFER_SIZE>>>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> InputDevice<SAMPLE_RATE, BUFFER_SIZE> {

	/// Find a new input device by name.
	/// Use 'default' as name to get the current default input device.
	pub fn new(device_name:&str) -> Result<Option<Self>, Box<dyn Error>> {
		let host:CpalHost = cpal::default_host();
		let device_name_lowercase:String = device_name.to_lowercase();

		// Find cpal device.
		let cpal_device:Option<CpalDevice> = {
			if device_name_lowercase == "default" {
				host.default_input_device()
			} else {
				cpal::default_host().input_devices()?.into_iter().find(|device| device.name().is_ok_and(|name| name.to_lowercase().contains(&device_name_lowercase)))
			}
		};

		// Return new device.
		Ok(
			match cpal_device {
				Some(cpal_device) => Some(InputDevice {
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
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> AudioGenerator for InputDevice<SAMPLE_RATE, BUFFER_SIZE> {

	/// Get the name of the generator.
	fn name(&self) -> &str {
		&self.name
	}
	
	/// Start the generator.
	fn start(&mut self) -> Result<(), Box<dyn Error>> {

		// Build config.
		let mut config:CpalStreamConfig = self.device.default_input_config()?.config();
		config.sample_rate = CpalSampleRate(SAMPLE_RATE);
		let is_stereo:bool = config.channels == 2;
		let buffer_handle_ref:Arc<Mutex<CircularBuffer<f32, BUFFER_SIZE>>> = Arc::clone(&self.buffer);

		// Build stream.
		let stream:CpalStream = self.device.build_input_stream(
			&config,
			move |data:&[f32], _| {
				let mut buffer_handle:MutexGuard<'_, CircularBuffer<f32, BUFFER_SIZE>> = buffer_handle_ref.lock().unwrap();
				if is_stereo {
					buffer_handle.extend(data);
				} else {
					let stereo_data:Vec<f32> = data.into_iter().map(|value| [*value; 2]).flatten().collect();
					buffer_handle.extend(&stereo_data);
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
	
	/// Stop the generator.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		self.stream = None;
		Ok(())
	}

	/// The amount of data currently available from the generator.
	fn amount_available(&self) -> usize {
		self.buffer.lock().unwrap().currently_stored()
	}

	/// Try to take an amount of data from the buffer.
	/// Returns None if the buffer does not contain enough data.
	fn take(&self, amount:usize) -> Option<Vec<f32>> {
		let mut buffer_handle:MutexGuard<'_, CircularBuffer<f32, BUFFER_SIZE>> = self.buffer.lock().unwrap();
		if buffer_handle.currently_stored() >= amount {
			Some(buffer_handle.take(amount))
		} else {
			None
		}
	}
}