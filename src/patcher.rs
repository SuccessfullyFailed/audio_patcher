use crate::{ audio_effect::{ AudioEffect, SizedAudioEffect }, audio_effects::VolumeAmplifier, device::{ InputDevice, OutputDevice }, display::PatcherDisplay, patcher_channel::{ PatcherChannel, PatcherChannelId } };
use std::{ error::Error, thread::sleep, time::{ Duration, Instant } };
use circular_buffer::{ CircularBufferMultiReadDyn, ReadCursor };
use mini_ini_parser::{ Ini, IniCategory };



pub struct Patcher<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	channels:Vec<Box<PatcherChannel<SAMPLE_RATE, BUFFER_SIZE>>>,
	channel_buffers:Vec<Box<CircularBufferMultiReadDyn<f32>>>,
	streams_running:bool,
	display:Option<PatcherDisplay>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> Patcher<SAMPLE_RATE, BUFFER_SIZE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new patcher.
	pub const fn new() -> Self {
		Patcher {
			channels: Vec::new(),
			channel_buffers: Vec::new(),
			streams_running: false,
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



	/// Set the given input device to a specific channel.
	pub fn set_channel_input_device(&mut self, channel_index:usize, device:InputDevice<SAMPLE_RATE, BUFFER_SIZE>) {
		self.modify_channel(channel_index, |channel| channel.set_input_device(device));
	}

	/// Find an input device by name and set it to a specific channel.
	/// Returns an error if the device could not be found.
	pub fn set_channel_input_device_by_name(&mut self, channel_index:usize, device_name:&str) -> Result<(), Box<dyn Error>> {
		if let Some(current_device) = self.channels[channel_index].input_device() {
			if current_device.name() == device_name {
				return Ok(());
			}
		}
		match InputDevice::new(device_name)? {
			Some(device) => {
				self.set_channel_input_device(channel_index, device);
				Ok(())
			},
			None => Err(format!("Patcher channel input device could not be set, no input device found by name '{device_name}'.").into())
		}
	}

	/// Set the given output device to a specific channel.
	pub fn set_channel_output_device(&mut self, channel_index:usize, device:OutputDevice<SAMPLE_RATE, BUFFER_SIZE>) {
		self.modify_channel(channel_index, |channel| channel.set_output_device(device));
	}

	/// Find an output device by name and set it to a specific channel.
	/// Returns an error if the device could not be found.
	pub fn set_channel_output_device_by_name(&mut self, channel_index:usize, device_name:&str) -> Result<(), Box<dyn Error>> {
		if let Some(current_device) = self.channels[channel_index].output_device() {
			if current_device.name() == device_name {
				return Ok(());
			}
		}
		match OutputDevice::new(device_name)? {
			Some(device) => {
				self.set_channel_output_device(channel_index, device);
				Ok(())
			},
			None => Err(format!("Patcher channel output device could not be set, no output device found by name '{device_name}'.").into())
		}
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
	pub fn add_effect_by_index<Effect:AudioEffect + 'static>(&mut self, channel_index:usize, effect_slot_index:usize, effect:Effect) {
		self.modify_channel(channel_index, |channel| channel.set_effect_to_slot(effect_slot_index, effect));
	}



	/// Update the entire patcher from settings.
	/// If anything goes wrong, an error will be returned and only the settings up to that point will be applied.
	pub fn update_from_settings(&mut self, settings:&Ini) -> Result<(), Box<dyn Error>> {
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
				if channel_settings["input_device"].is_ok() {
					self.set_channel_input_device_by_name(channel_index, &channel_settings["input_device"].value)?;
				}
				if channel_settings["output_device"].is_ok() {
					self.set_channel_output_device_by_name(channel_index, &channel_settings["output_device"].value)?;
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
						let mut effect_settings:Vec<(String, f32)> = Vec::new();
						for setting_variable in channel_settings.data.iter().filter(|var| var.name.starts_with(&effect_settings_prefix)) {
							effect_settings.push((setting_variable.name.replace(&effect_settings_prefix, ""), setting_variable.value.parse()?));
						}

						// Create target effect.
						let created_effect:Option<VolumeAmplifier> = {
							match channel_settings[&effect_key].value.as_str() {
								VolumeAmplifier::NAME => Some(VolumeAmplifier::default().with_settings(&effect_settings)),
								_ => None
							}
						};

						// If effect does not exist yet, set it.
						if let Some(created_effect) = created_effect {
							self.add_effect_by_index(channel_index, effect_slot_index, created_effect);
						}
					}
				}
			}
		}

		// Return success.
		Ok(())
	}

	/// Start all streams of devices.
	pub fn start_streams(&mut self) -> Result<(), Box<dyn Error>> {
		for channel in &mut *self.channels {
			if let Some(device) = channel.input_device_mut() {
				device.create_stream()?;
			}
			if let Some(device) = channel.output_device_mut() {
				device.create_stream()?;
			}
		}
		self.streams_running = true;
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
				sleep(interval - duration_since_last_interval);
			}
			last_interval = now;

			// Update buffers from right to left.
			self.update()?;
		}
	}

	/// Update the patcher once, updating all channels.
	/// Updates from right to left to make sure parents update their buffer first, allowing it to be used by children.
	pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
		if !self.streams_running {
			self.start_streams()?;
		}

		// Update each channel separately.
		let mut peaks:Vec<[f32; 2]> = vec![[0.0; 2]; self.channels.len()];
		for (patcher_channel_index, patcher_channel) in self.channels.iter_mut().enumerate().rev() {
			if patcher_channel.is_idle() {
				continue;
			}

			// For this channel, create a buffer from all connected sources and apply all effects.
			let mut channel_buffer:Vec<f32> = patcher_channel.get_combined_input_buffer(&mut *self.channel_buffers);
			for effect in patcher_channel.effects_mut() {
				effect.apply_to_buffer(&mut channel_buffer);
			}

			// Find out the peaks of the new buffer for the display.
			if self.display.is_some() {
				for (sample_index, sample) in channel_buffer.iter().enumerate() {
					let channel_index:usize = sample_index & 1;
					let sample_abs:f32 = sample.abs();
					if sample_abs > peaks[patcher_channel_index][channel_index] {
						peaks[patcher_channel_index][channel_index] = sample_abs;
					}
				}
			}

			// Write the final buffer to the channel's output buffer and output device buffer.
			self.channel_buffers[patcher_channel_index].extend(&channel_buffer);
			if let Some(output_device) = patcher_channel.output_device_mut() {
				output_device.write_to_buffer(&channel_buffer);
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