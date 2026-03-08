use crate::{ audio_effect::{ AudioEffect, AudioEffectPlaceHolder }, audio_generator::AudioGenerator, audio_endpoint::AudioEndPoint };
use circular_buffer::{ CircularBufferMultiReadDyn, ReadCursor };
use std::error::Error;



#[derive(PartialEq, Clone)]
pub struct PatcherChannelId {
	pub(crate) index:usize,
	pub(crate) name:String
}
impl PatcherChannelId {
	pub fn new(index:usize, name:&str) -> PatcherChannelId {
		PatcherChannelId {
			index: index,
			name: name.to_string()
		}
	}
}



pub struct PatcherChannel<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> {
	id:PatcherChannelId,
	volume:f32,
	connections:Vec<(PatcherChannelId, ReadCursor)>,
	generator:Option<Box<dyn AudioGenerator>>,
	effects:Vec<Box<dyn AudioEffect>>,
	end_point:Option<Box<dyn AudioEndPoint>>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> PatcherChannel<SAMPLE_RATE, BUFFER_SIZE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new channel.
	pub fn new(channel_index:usize, channel_name:&str) -> Self {
		PatcherChannel {
			id: PatcherChannelId::new(channel_index, channel_name),
			volume: 1.0,
			connections: Vec::new(),
			generator: None,
			effects: Vec::new(),
			end_point: None
		}
	}

	/// Set the volume of this channel.
	pub fn set_volume(&mut self, volume:f32) {
		self.volume = volume;
	}

	/// Set a generator.
	pub fn set_generator<Generator:AudioGenerator + 'static>(&mut self, generator:Generator) {
		self.generator = Some(Box::new(generator));
	}

	/// Set an audio endpoint.
	pub fn set_end_point<EndPoint:AudioEndPoint + 'static>(&mut self, end_point:EndPoint) {
		self.end_point = Some(Box::new(end_point));
	}

	/// Add a connection to another channel.
	pub fn add_connection(&mut self, channel_id:&PatcherChannelId, patcher_buffer_cursor:ReadCursor) -> Result<(), Box<dyn Error>> {
		if channel_id.index < self.id.index {
			Err(format!("Cannot create connection from channel {} to channel {}, can only create connections with higher indexes.", self.id.index, channel_id.index).into())
		} else if self.connections.iter().any(|(connection_endpoint, _)| connection_endpoint == channel_id) {
			Ok(())
		} else {
			self.connections.push((channel_id.clone(), patcher_buffer_cursor));
			Ok(())
		}
	}

	/// Set an effect to a specific slot.
	pub fn set_effect_to_slot<Effect:AudioEffect + 'static>(&mut self, slot_index:usize, effect:Effect) {
		if self.effects.len() <= slot_index {
			for _ in self.effects.len()..slot_index + 1 {
				self.effects.push(Box::new(AudioEffectPlaceHolder::new()));
			}
		}
		self.effects[slot_index] = Box::new(effect);
	}



	/* PROPERTY GETTER METHODS */

	/// Get the ID of the channel.
	pub fn id(&self) -> &PatcherChannelId {
		&self.id
	}

	/// Wether or not this channel is doing anything. Returns true if the channel does not alter or generate any audio.
	pub fn is_idle(&self) -> bool {
		self.generator.is_none() && self.end_point.is_none() && self.connections.is_empty() && self.effects.iter().all(|effect| effect.is_placeholder())
	}

	/// Get a reference to the generator of this channel.
	/// Returns None if no device is set.
	pub fn generator(&self) -> &Option<Box<dyn AudioGenerator>> {
		&self.generator
	}

	/// Get a mutable reference to the generator of this channel.
	/// Returns None if no device is set.
	pub fn generator_mut(&mut self) -> &mut Option<Box<dyn AudioGenerator>> {
		&mut self.generator
	}

	/// Get a reference to the endpoint of this channel.
	/// Returns None if no device is set.
	pub fn end_point(&self) -> &Option<Box<dyn AudioEndPoint>> {
		&self.end_point
	}

	/// Get a mutable reference to the endpoint of this channel.
	/// Returns None if no device is set.
	pub fn end_point_mut(&mut self) -> &mut Option<Box<dyn AudioEndPoint>> {
		&mut self.end_point
	}

	/// Get a mutable reference to the effects on this channel.
	pub fn effects_mut(&mut self) -> &mut Vec<Box<dyn AudioEffect>> {
		&mut self.effects
	}



	/* USAGE METHODS */

	/// Create a buffer from this channel's input buffer with all effects applied.
	pub fn get_processed_buffer(&mut self, patcher_buffers:&mut [Box<CircularBufferMultiReadDyn<f32>>], max_batch_size:usize) -> Vec<f32> {

		// If absolutely nothing is happening, return empty list.
		if self.is_idle() {
			return Vec::new();
		}

		// Get initial buffer.
		let has_inputs:bool = self.generator.is_some() || !self.connections.is_empty();
		let has_active_effects:bool = self.effects.iter().any(|effect| !effect.is_placeholder());
		let mut buffer:Vec<f32> = {
			if has_inputs {
				self.get_combined_input_buffer(patcher_buffers, max_batch_size)
			} else if has_active_effects {
				vec![0.0; max_batch_size]
			} else {
				Vec::new()
			}
		};

		// Apply channel built-in effects.
		if self.volume != 1.0 {
			for sample in &mut buffer {
				*sample *= self.volume;
			}
		}

		// Apply manual effects.
		for effect in &mut self.effects {
			effect.apply_to_buffer(&mut buffer);
		}

		// Normalize buffer, make sure the buffers don't exceed max volume.
		for sample in &mut buffer {
			if *sample > 1.0 {
				*sample = 1.0;
			} else if *sample < -1.0 {
				*sample = -1.0;
			}
		}

		// Return fully processed buffer.
		buffer
	}

	/// Create a buffer from this channel's input device and connections combined.
	fn get_combined_input_buffer(&mut self, patcher_buffers:&mut [Box<CircularBufferMultiReadDyn<f32>>], max_batch_size:usize) -> Vec<f32> {

		// Determine batch size.
		let batch_size:usize = {
			let smallest_connection_available:Option<usize> = self.connections.iter().map(|(connection, cursor)| patcher_buffers[connection.index].currently_stored(cursor)).min();
			let generator_buffer_available:Option<usize> = self.generator.as_ref().map(|device| device.amount_available());
			[smallest_connection_available, generator_buffer_available].into_iter().flatten().min().unwrap_or_default().min(max_batch_size)
		};
		if batch_size == 0 {
			return Vec::new();
		}

		// Get buffer from input device and connected channels.
		let generator_buffer:Option<Vec<f32>> = match self.generator_mut() { Some(device) => device.take(batch_size), None => None };
		let mut connection_buffers:Vec<Vec<f32>> = Vec::new();
		for (connection, buffer_cursor) in &self.connections {
			connection_buffers.push(patcher_buffers[connection.index].take(batch_size, buffer_cursor));
		}

		// Combine received buffers.
		let combined_buffer:Vec<f32> = {
			if connection_buffers.is_empty() {
				generator_buffer.unwrap_or_default()
			} else {
				let mut combined_buffer:Vec<f32> = generator_buffer.unwrap_or(connection_buffers.remove(connection_buffers.len() - 1));
				let longest_buffer_len:usize = combined_buffer.len().max(connection_buffers.iter().map(|buffer| buffer.len()).max().unwrap_or_default());
				combined_buffer.extend(vec![0.0; longest_buffer_len - combined_buffer.len()]);

				// Looping through sample per index, then buffer index feels logical, but looping through entire buffers is more favorable with CPU cache.
				for additional_buffer in connection_buffers {
					for index in 0..additional_buffer.len() {
						combined_buffer[index] += additional_buffer[index];
					}
				}

				combined_buffer
			}
		};

		// Return combined buffer.
		combined_buffer
	}
}