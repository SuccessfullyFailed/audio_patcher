use std::{ error::Error, sync::{ Mutex, MutexGuard }, thread::{ self, sleep }, time::Duration };
use file_ref::{ DirMonitor, FileRef };
use crate::audio_effect::AudioEffect;
use audio_buffer::AudioBuffer;



static BUFFERS:Mutex<Vec<(Vec<f32>, usize)>> = Mutex::new(Vec::new());



pub struct SoundBoard {
	source_dir:Option<FileRef>,
	is_listening:bool,
	buffer_index:Option<usize>,
	sample_rate:u32
}
impl SoundBoard {
	pub const NAME:&str = "sound_board";
}
impl AudioEffect for SoundBoard {

	fn initialize(&mut self, sample_rate:u32) {
		self.sample_rate = sample_rate;
	}


	fn apply_to_buffer(&mut self, buffer:&mut [f32]) {

		// Get buffer index.
		let buffer_index:usize = {
			match self.buffer_index {
				Some(index) => index,
				None => {
					let mut buffers_handle:MutexGuard<'_, Vec<(Vec<f32>, usize)>> = BUFFERS.lock().unwrap();
					let buffer_index:usize = buffers_handle.len();
					buffers_handle.push((Vec::new(), 0));
					self.buffer_index = Some(buffer_index);
					buffer_index
				}
			}
		};

		// Start listener.
		if !self.is_listening {
			if let Some(source_dir) = self.source_dir.clone() {
				let sample_rate:u32 = self.sample_rate;
				thread::spawn(move || {
					let monitor_launch:Result<(), Box<dyn Error>> = DirMonitor::new(source_dir.path()).with_add_handler(move |file| add_file_to_queue(file, buffer_index, sample_rate)).run();
					if let Err(error) = monitor_launch {
						eprintln!("SoundBoard could not launch DirMonitor: {error}");
					}
				});
				self.is_listening = true;
			}
		}

		// If there are any files in the buffer, add them to the buffer.
		if let Some(buffer_index) = self.buffer_index {
			let (soundboard_buffer, buffer_cursor) = &mut BUFFERS.lock().unwrap()[buffer_index];
			if !soundboard_buffer.is_empty() {
				if *buffer_cursor >= soundboard_buffer.len() {
					*soundboard_buffer = Vec::new();
					*buffer_cursor = 0;
				} else {
					let amount_to_take:usize = buffer.len().min(soundboard_buffer.len() - *buffer_cursor);
					for index in 0..amount_to_take {
						buffer[index] += soundboard_buffer[*buffer_cursor + index];
					}
					*buffer_cursor += amount_to_take;
				}
			}
		}
	}

	fn set_setting(&mut self, name:&str, value:&str) {
		if name == "source_dir" {
			let target_dir:FileRef = FileRef::new(value);
			if !target_dir.exists() {
				if let Err(error) = target_dir.create_dir() {
					eprintln!("Could not create soundboard dir '{}'. {}.", target_dir, error);
				}
			}
			self.source_dir = Some(target_dir);
		}
	}
}
impl Default for SoundBoard {
	fn default() -> Self {
		SoundBoard {
			source_dir: None,
			is_listening: false,
			buffer_index: None,
			sample_rate: 1
		}
	}
}


fn add_file_to_queue(file:&FileRef, buffer_index:usize, sample_rate:u32) {

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

	// Read the file and push it to the buffer.
	match AudioBuffer::from_wav(file.path()) {
		Ok(mut audio_buffer) => {
			audio_buffer.resample(2, sample_rate);
			BUFFERS.lock().unwrap()[buffer_index].0.extend_from_slice(audio_buffer.data());
		},
		Err(error) => eprintln!("Could not read audio from file '{file}'. {error}")
	}
	let _ = file.delete();
}