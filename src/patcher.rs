use crate::{ SoundBoard, audio_effect::{ AudioEffect, SizedAudioEffect }, audio_effects::VolumeAmplifier, audio_endpoint::AudioEndPoint, audio_endpoints::OutputDevice, audio_generator::AudioGenerator, audio_generators::InputDevice, display::PatcherDisplay, patcher_channel::{ PatcherChannel, PatcherChannelId } };
use std::{ error::Error, thread::sleep, time::{ Duration, Instant } };
use circular_buffer::{ CircularBufferMultiReadDyn, ReadCursor };
use mini_ini_parser::{ Ini, IniCategory };



pub struct Patcher<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	channels:Vec<Box<PatcherChannel<SAMPLE_RATE, BUFFER_SIZE>>>,
	channel_buffers:Vec<Box<CircularBufferMultiReadDyn<f32>>>,
	initialized:bool,
	display:Option<PatcherDisplay>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> Patcher<SAMPLE_RATE, BUFFER_SIZE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new patcher.
	pub const fn new() -> Self {
		Patcher {
			channels: Vec::new(),
			channel_buffers: Vec::new(),
			initialized: false,
			display: None
		}
	}



	/* USAGE METHODS */

	/// Enable display.
	pub fn add_display(&mut self) -> Result<(), Box<dyn Error>> {
		self.display = Some(PatcherDisplay::new()?);
		Ok(())
	}

	/// Find a channel's index by name.
	fn channel_index_by_name(&self, name:&str) -> Option<usize> {
		self.channels.iter().position(|channel| channel.id().name == name)
	}

	/// Find a channel's index by name. Returns a results instead of an option.
	fn channel_index_by_name_r(&self, name:&str) -> Result<usize, Box<dyn Error>> {
		match self.channel_index_by_name(name) {
			Some(index) => Ok(index),
			None => Err(format!("Could not find channel by name '{name}'.").into())
		}
	}



	/* MODIFICATION METHODS */

	/// Make sure a channel with the given index exists.
	/// Will create it if it does not.
	fn ensure_channel(&mut self, channel_index:usize) {
		while self.channels.len() <= channel_index {
			self.channels.push(Box::new(PatcherChannel::new(self.channels.len(), "")));
		}
		while self.channel_buffers.len() <= channel_index {
			self.channel_buffers.push(Box::new(CircularBufferMultiReadDyn::new(BUFFER_SIZE)));
		}
	}

	/// Make a modification for a specific channel by index.
	/// If the channel is not initialized, it will initialize it.
	/// Does nothing if the index is invalid.
	fn modify_channel<Modification:FnOnce(&mut PatcherChannel<SAMPLE_RATE, BUFFER_SIZE>)>(&mut self, channel_index:usize, modification:Modification) {
		self.ensure_channel(channel_index);
		modification(&mut self.channels[channel_index]);
	}

	/// Make a modification for a specific channel by index.
	/// If the channel is not initialized, it will initialize it.
	/// Will return an error if the index is invalid.
	fn modify_channel_r<Modification:FnOnce(&mut PatcherChannel<SAMPLE_RATE, BUFFER_SIZE>) -> Result<(), Box<dyn Error>>>(&mut self, channel_index:usize, modification:Modification) -> Result<(), Box<dyn Error>> {
		self.ensure_channel(channel_index);
		modification(&mut self.channels[channel_index])
	}



	/// Set the given generator to a specific channel.
	pub fn set_channel_generator<Generator:AudioGenerator + 'static>(&mut self, channel_index:usize, device:Generator) {
		self.modify_channel(channel_index, |channel| channel.set_generator(device));
	}

	/// Find or create a generator by name and set it to a specific channel.
	/// Returns an error if the device could not be found.
	pub fn set_channel_generator_by_name(&mut self, channel_index:usize, generator_name:&str) -> Result<(), Box<dyn Error>> {

		// If generator exists with this name, simply return.
		if self.channels[channel_index].generator().as_ref().is_some_and(|generator| generator.name() == generator_name) {
			return Ok(());
		}

		// Try to build a soundboard.
		const SOUNDBOARD_TAG:&str = "soundboard:";
		if generator_name.to_lowercase().starts_with(SOUNDBOARD_TAG) {
			let source_dir:&str = &generator_name[SOUNDBOARD_TAG.len()..];
			self.set_channel_generator(channel_index, SoundBoard::<SAMPLE_RATE>::new(source_dir));
			return Ok(());
		}

		// If an input-device can be found with this name, set that as generator.
		if let Some(device) = InputDevice::<SAMPLE_RATE, BUFFER_SIZE>::new(generator_name)? {
			self.set_channel_generator(channel_index, device);
			return Ok(());
		}
		
		// Nothing could be created, return error.
		Err(format!("Patcher channel generator could not be set, no generators found by name '{generator_name}'.").into())
	}

	/// Set the given endpoint to a specific channel.
	pub fn set_channel_end_point<EndPoint:AudioEndPoint + 'static>(&mut self, channel_index:usize, device:EndPoint) {
		self.modify_channel(channel_index, |channel| channel.set_end_point(device));
	}

	/// Find an endpoint by name and set it to a specific channel.
	/// Returns an error if the device could not be found.
	pub fn set_channel_end_point_by_name(&mut self, channel_index:usize, end_point_name:&str) -> Result<(), Box<dyn Error>> {

		// If endpoint exists with this name, simply return.
		if self.channels[channel_index].end_point().as_ref().is_some_and(|end_point| end_point.name() == end_point_name) {
			return Ok(());
		}

		// If an output-device can be found with this name, set that as endpoint.
		if let Some(device) = OutputDevice::<SAMPLE_RATE, BUFFER_SIZE>::new(end_point_name)? {
			self.set_channel_end_point(channel_index, device);
			return Ok(());
		}
		
		// Nothing could be created, return error.
		Err(format!("Patcher channel endpoint could not be set, no generators found by name '{end_point_name}'.").into())
	}



	/// Add a connection from one channel to another by channel names.
	/// Returns an error if any of the channels could not be found or the connection failed.
	pub fn add_connection_by_name(&mut self, source_channel_name:&str, target_channel_name:&str) -> Result<(), Box<dyn Error>> {
		self.add_connection_by_index(
			self.channel_index_by_name_r(source_channel_name)?,
			self.channel_index_by_name_r(target_channel_name)?
		)
	}

	/// Add a connection from one channel to another by channel indexes.
	/// Returns an error if any of the channels could not be found or the connection failed.
	pub fn add_connection_by_index(&mut self, source_channel_index:usize, target_channel_index:usize) -> Result<(), Box<dyn Error>> {
		if source_channel_index == target_channel_index {
			Err(format!("Could not create connection from channel {source_channel_index} to  {target_channel_index}. Cannot create connections to own channel.").into())
		} else if source_channel_index > target_channel_index {
			Err(format!("Could not create connection from channel {source_channel_index} to  {target_channel_index}. Cannot create connections to lower channel.").into())
		} else {
			self.ensure_channel(target_channel_index);
			let target_channel_id:PatcherChannelId = self.channels[target_channel_index].id().clone();
			let cursor:ReadCursor = self.channel_buffers[target_channel_id.index].create_read_cursor();
			self.modify_channel_r(source_channel_index, |channel| channel.add_connection(&target_channel_id, cursor))
		}
	}



	/// Add an audio-effect in a specific slot of a specific channel.
	pub fn set_effect_to_slot<Effect:AudioEffect + 'static>(&mut self, channel_index:usize, effect_slot_index:usize, effect:Effect) {
		self.modify_channel(channel_index, |channel| channel.set_effect_to_slot(effect_slot_index, effect));
	}



	/// Update the entire patcher from settings.
	/// If anything goes wrong, an error will be returned and only the settings up to that point will be applied.
	pub fn update_from_ini(&mut self, settings:&Ini) -> Result<(), Box<dyn Error>> {
		const MAX_CHANNEL_INDEX:usize = 512;
		const MAX_EFFECT_INDEX:usize = 64;

		// Update channels from right to left, as connections can only be made to the right.
		// The target channel will have to be initialized before the connection can be made.
		for channel_index in (0..MAX_CHANNEL_INDEX).rev() {
			let channel_settings:&IniCategory = &settings[&format!("channel_{channel_index}")];
			if channel_settings.is_ok() {
				let channel_name:&str = if channel_settings["name"].is_ok() { &channel_settings["name"].value } else { "" };
				self.ensure_channel(channel_index);
				self.channels[channel_index] = Box::new(PatcherChannel::new(channel_index, channel_name));

				// Add built-in effects.
				if channel_settings["volume"].is_ok() {
					let channel_volume:f32 = channel_settings["volume"].value.parse()?;
					self.modify_channel(channel_index, |channel| channel.set_volume(channel_volume));
				}

				// Add input and output device if defined.
				if channel_settings["generator"].is_ok() {
					self.set_channel_generator_by_name(channel_index, &channel_settings["generator"].value)?;
				}
				if channel_settings["end_point"].is_ok() {
					self.set_channel_end_point_by_name(channel_index, &channel_settings["end_point"].value)?;
				}

				// Add Connections to other channels if defined.
				if channel_settings["connections"].is_ok() {
					for connection_channel_name in channel_settings["connections"].value.split(",").map(|name| name.trim()).filter(|name| !name.is_empty()) {
						self.add_connection_by_name(&channel_name, connection_channel_name)?;
					}
				}

				// Add effects if defined.
				for effect_slot_index in 0..MAX_EFFECT_INDEX {
					let effect_key:String = format!("effect_{effect_slot_index}");
					if channel_settings[&effect_key].is_ok() {

						// Collect effect setting.
						let effect_settings_prefix:String = effect_key.clone() + ".";
						let mut effect_settings:Vec<(&str, &str)> = Vec::new();
						for setting_variable in channel_settings.data.iter().filter(|var| var.name.starts_with(&effect_settings_prefix)) {
							effect_settings.push((&setting_variable.name[effect_settings_prefix.len()..], &setting_variable.value));
						}

						// Create target effect.
						let created_effect:Option<Box<dyn AudioEffect>> = {
							match channel_settings[&effect_key].value.as_str() {
								VolumeAmplifier::NAME => Some(Box::new(VolumeAmplifier::default().with_settings(&effect_settings))),
								_ => None
							}
						};

						// If effect does not exist yet, set it.
						if let Some(created_effect) = created_effect {
							self.set_effect_to_slot(channel_index, effect_slot_index, created_effect);
						}
					}
				}
			}
		}

		// Return success.
		Ok(())
	}

	/// Initialize and start the system, making some preparations.
	pub fn start(&mut self) -> Result<(), Box<dyn Error>> {

		for channel in &mut *self.channels {
			
			// Start all generators and endpoints.
			if let Some(device) = channel.end_point_mut() {
				device.start()?;
			}
			if let Some(device) = channel.generator_mut() {
				device.start()?;
			}

			// Initialize effects.
			for effect in channel.effects_mut() {
				effect.initialize(SAMPLE_RATE);
			}

			// Flush all buffers.
			for buffer in &mut self.channel_buffers {
				buffer.flush();
			}
		}

		self.initialized = true;
		Ok(())
	}

	/// Stop the system, making sure the entire system starts idling.
	pub fn stop(&mut self) -> Result<(), Box<dyn Error>> {

		for channel in &mut *self.channels {
			
			// Start all generators and endpoints.
			if let Some(device) = channel.generator_mut() {
				device.stop()?;
			}
			if let Some(device) = channel.end_point_mut() {
				device.stop()?;
			}
		}

		self.initialized = false;
		Ok(())
	}

	/// Run the patcher, continuously updating all channels.
	/// Runs forever or until panicking.
	pub fn run(&mut self, interval:Duration) -> Result<(), Box<dyn Error>> {
		let mut last_interval:Instant = Instant::now() - interval;
		loop {

			// Adhere to interval.
			let now:Instant = Instant::now();
			let duration_since_last_interval:Duration = now.duration_since(last_interval);
			if duration_since_last_interval < interval {
				let sleep_duration:Duration = interval - duration_since_last_interval;
				sleep(sleep_duration);
				last_interval = now + sleep_duration;
			} else {
				last_interval = now;
			}

			// Update buffers from right to left.
			self.update()?;
		}
	}

	/// Update the patcher once, updating all channels.
	/// Updates from right to left to make sure parents update their buffer first, allowing it to be used by children.
	/// The ideal batch size is used for channels that have effects, but no input device.
	/// These effects need to handle on an initial silent audio, which has to be generated.
	/// To do this, the batch size for this silent audio is required.
	pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
		if !self.initialized {
			self.start()?;
		}

		// Update each channel separately.
		let channel_is_idle:Vec<bool> = self.channels.iter().map(|channel| channel.is_idle()).collect();
		let mut peaks:Vec<[f32; 2]> = vec![[0.0; 2]; self.channels.len()];
		for (patcher_channel_index, patcher_channel) in self.channels.iter_mut().enumerate().rev() {
			if patcher_channel.is_idle() {
				continue;
			}

			// Get the fully processed buffer for this channel.
			let space_in_buffer:usize = self.channel_buffers[patcher_channel_index].available_storage();
			let channel_buffer:Vec<f32> = patcher_channel.get_processed_buffer(&mut self.channel_buffers, &channel_is_idle, space_in_buffer);

			// Find out the peaks of the new buffer for the display.
			if self.display.is_some() {
				for (sample_index, sample) in channel_buffer.iter().enumerate() {
					let left_or_right:usize = sample_index & 1;
					let sample_abs:f32 = sample.abs();
					if sample_abs > peaks[patcher_channel_index][left_or_right] {
						peaks[patcher_channel_index][left_or_right] = sample_abs;
					}
				}
			}

			// Write the final buffer to the channel's output buffer and output device buffer.
			self.channel_buffers[patcher_channel_index].extend(&channel_buffer);
			if let Some(end_point) = patcher_channel.end_point_mut() {
				end_point.extend(&channel_buffer);
			}
		}

		// Update the display if it exists.
		if let Some(display) = &mut self.display {
			display.update(peaks)?;
			if !display.is_open() {
				self.display = None;
			}
		}

		Ok(())
	}
}