use std::{ error::Error, sync::{ Arc, Mutex, MutexGuard }, thread::{ self, JoinHandle, sleep }, time::Duration };
use file_ref::{ DirMonitor, FileRef };
use audio_buffer::AudioBuffer;
use crate::AudioGenerator;



pub struct SoundBoard<const SAMPLE_RATE:u32> {
	source_dir:FileRef,
	listener_thread:Option<JoinHandle<()>>,
	buffers:Arc<Mutex<Vec<(Vec<f32>, usize)>>>,
	enabled:bool
}
impl<const SAMPLE_RATE:u32> SoundBoard<SAMPLE_RATE> {
	
	/// Create a new soundboard.
	pub fn new(source_dir:&str) -> SoundBoard<SAMPLE_RATE> {
		SoundBoard {
			source_dir: FileRef::new(source_dir),
			listener_thread: None,
			buffers: Arc::new(Mutex::new(Vec::new())),
			enabled: false
		}
	}

	/// Add a file to the static queue at the given buffer index.
	fn add_file_to_queue(file:&FileRef, buffers:&Arc<Mutex<Vec<(Vec<f32>, usize)>>>) {

		// Wait for file to be usable.
		const INTERVAL:Duration = Duration::from_millis(10);
		const MAX_ATTEMPTS:usize = 100;
		for _ in 0..MAX_ATTEMPTS {
			if file.is_accessible() {
				break;
			} else {
				sleep(INTERVAL);
			}
		}
		if !file.is_accessible() {
			eprintln!("Could not load file '{file}' into soundboard, file is not accessible.");
		}
		
		
		// Read the file and push it to the buffer.
		match AudioBuffer::from_wav(file.path()) {
			Ok(mut buffer_from_file) => {
				buffer_from_file.resample(2, SAMPLE_RATE);
				buffers.lock().unwrap().push((buffer_from_file.data().to_vec(), 0));
			},
			Err(error) => eprintln!("Could not play audio through soundboard: {error}")
		}

		// Delete file and return success.
		let _ = file.delete();
	}
}
impl<const SAMPLE_RATE:u32> AudioGenerator for SoundBoard<SAMPLE_RATE> {

	/// Get the name of the generator.
	fn name(&self) -> &str {
		"Soundboard"
	}
	
	/// Start the generator.
	fn start(&mut self) -> Result<(), Box<dyn Error>> {

		// Ensure source dir exists.
		if !self.source_dir.exists() {
			self.source_dir.create_dir()?;
		}

		// Start listener thread.
		let source_dir:FileRef = self.source_dir.clone();
		let monitor_buffers:Arc<Mutex<Vec<(Vec<f32>, usize)>>> = Arc::clone(&self.buffers);
		if self.listener_thread.is_none() {
			self.listener_thread = Some(
				thread::spawn(move || {
					let monitor_launch:Result<(), Box<dyn Error>> = DirMonitor::new(source_dir.path()).with_add_handler(move |file| Self::add_file_to_queue(file, &monitor_buffers)).run();
					if let Err(error) = monitor_launch {
						eprintln!("SoundBoard could not launch DirMonitor: {error}");
					}
				})
			);
		}

		// Return success.
		self.enabled = true;
		Ok(())
	}

	/// Stop the generator.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		self.enabled = false;
		Ok(())
	}

	/// The amount of data currently available from the generator.
	fn amount_available(&self) -> usize {
		// If any buffers are still in use, return a very high amount available.
		// This makes sure no trails of audio are left unplayed until the next buffer is added.
		if self.enabled && self.buffers.lock().unwrap().is_empty() {
			0
		} else {
			SAMPLE_RATE as usize
		}
	}

	/// Try to take an amount of data from the buffer.
	/// Returns None if the buffer does not contain enough data.
	fn take(&self, amount:usize) -> Option<Vec<f32>> {
		if !self.enabled {
			return None;
		}
		let mut buffer_handle:MutexGuard<'_, Vec<(Vec<f32>, usize)>> = self.buffers.lock().unwrap();
		if buffer_handle.is_empty() {
			return None;
		}

		// Get buffer for each sound in the buffer.
		let mut output_buffers:Vec<Vec<f32>> = Vec::with_capacity(buffer_handle.len());
		let mut buffer_index:usize = 0;
		while buffer_index < buffer_handle.len() {
			let (buffer_data, cursor) = &mut buffer_handle[buffer_index];
			let amount_available:usize = amount.min(buffer_data.len() - *cursor);
			output_buffers.push(buffer_data[*cursor..*cursor + amount_available].to_vec());
			*cursor += amount_available;

			if *cursor < buffer_data.len() {
				buffer_index += 1;
			} else {
				buffer_handle.remove(buffer_index);
			}
		}

		// Combine the buffers into one output buffer.
		if output_buffers.is_empty() {
			None
		} else {
			let mut combined_buffer:Vec<f32> = output_buffers.remove(output_buffers.len() - 1);
			if combined_buffer.len() < amount {
				combined_buffer.extend(vec![0.0; amount - combined_buffer.len()]);
			}
			for additional_buffer in output_buffers {
				for sample_index in 0..additional_buffer.len() {
					combined_buffer[sample_index] += additional_buffer[sample_index];
				}
			}
			Some(combined_buffer)
		}
	}
}