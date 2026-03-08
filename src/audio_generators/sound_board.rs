use std::{ error::Error, sync::{ Mutex, MutexGuard }, thread::{ self, JoinHandle, sleep }, time::Duration };
use file_ref::{ DirMonitor, FileRef };
use audio_buffer::AudioBuffer;
use crate::AudioGenerator;



static SOUNDBOARD_BUFFERS:Mutex<Vec<(Vec<f32>, usize, bool)>> = Mutex::new(Vec::new());



pub struct SoundBoard<const SAMPLE_RATE:u32> {
	source_dir:FileRef,
	listener_thread:Option<JoinHandle<()>>,
	buffer_index:usize
}
impl<const SAMPLE_RATE:u32> SoundBoard<SAMPLE_RATE> {
	
	/// Create a new soundboard.
	pub fn new(source_dir:&str) -> SoundBoard<SAMPLE_RATE> {
		SoundBoard {
			source_dir: FileRef::new(source_dir),
			listener_thread: None,
			buffer_index: {
				let mut buffers_handle:MutexGuard<'_, Vec<(Vec<f32>, usize, bool)>> = SOUNDBOARD_BUFFERS.lock().unwrap();
				buffers_handle.push((Vec::new(), 0, false));
				buffers_handle.len() - 1
			}
		}
	}

	/// Add a file to the static queue at the given buffer index.
	fn add_file_to_queue(file:&FileRef, buffer_index:usize) {

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
		if SOUNDBOARD_BUFFERS.lock().unwrap()[buffer_index].2 {
			match AudioBuffer::from_wav(file.path()) {
				Ok(mut buffer_from_file) => {
					buffer_from_file.resample(2, SAMPLE_RATE);
					SOUNDBOARD_BUFFERS.lock().unwrap()[buffer_index].0.extend_from_slice(buffer_from_file.data());
				},
				Err(error) => eprintln!("Could not play audio through soundboard: {error}")
			}
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
		let buffer_index:usize = self.buffer_index;
		if self.listener_thread.is_none() {
			self.listener_thread = Some(
				thread::spawn(move || {
					let monitor_launch:Result<(), Box<dyn Error>> = DirMonitor::new(source_dir.path()).with_add_handler(move |file| Self::add_file_to_queue(file, buffer_index)).run();
					if let Err(error) = monitor_launch {
						eprintln!("SoundBoard could not launch DirMonitor: {error}");
					}
				})
			);
		}

		// Set buffer as active.
		SOUNDBOARD_BUFFERS.lock().unwrap()[self.buffer_index].2 = true;

		// Return success.
		Ok(())
	}

	/// Stop the generator.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		self.listener_thread = None;
		SOUNDBOARD_BUFFERS.lock().unwrap()[self.buffer_index] = (Vec::new(), 0, false);
		Ok(())
	}

	/// The amount of data currently available from the generator.
	fn amount_available(&self) -> usize {
		let (data, cursor, enabled) = &mut SOUNDBOARD_BUFFERS.lock().unwrap()[self.buffer_index];
		if *enabled {
			data.len() - *cursor
		} else {
			0
		}
	}

	/// Try to take an amount of data from the buffer.
	/// Returns None if the buffer does not contain enough data.
	fn take(&self, amount:usize) -> Option<Vec<f32>> {
		let (data, cursor, enabled) = &mut SOUNDBOARD_BUFFERS.lock().unwrap()[self.buffer_index];
		let available:usize = data.len() - *cursor;
		if !*enabled || available == 0  {
			None
		} else if available > amount {
			let output:Vec<f32> = data[*cursor..*cursor + amount].to_vec();
			*cursor += amount;
			Some(output)
		} else {
			let mut output:Vec<f32> = data[*cursor..*cursor + amount].to_vec();
			output.extend(vec![0.0; amount - available]);
			*data = Vec::new();
			*cursor = 0;
			Some(output)
		}
	}
}