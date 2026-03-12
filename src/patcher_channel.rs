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
		self.generator.as_ref().is_none_or(|generator| generator.is_idle()) &&
		self.end_point.as_ref().is_none_or(|end_point| end_point.is_idle()) &&
		self.connections.is_empty() && // Not exactly accurate as a connection could be idle
		self.effects.iter().all(|effect| effect.is_idle())
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
	pub fn get_processed_buffer(&mut self, patcher_buffers:&mut [Box<CircularBufferMultiReadDyn<f32>>], channel_is_idle:&[bool], max_batch_size:usize) -> Vec<f32> {

		// If absolutely nothing is happening, return empty list.
		if self.is_idle() {
			return Vec::new();
		}

		// Get initial buffer.
		let mut buffer:Vec<f32> = self.get_combined_input_buffer(patcher_buffers, channel_is_idle, max_batch_size);
		if buffer.len() == 0 {
			return Vec::new();
		}

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
	fn get_combined_input_buffer(&mut self, patcher_buffers:&mut [Box<CircularBufferMultiReadDyn<f32>>], channel_is_idle:&[bool], max_batch_size:usize) -> Vec<f32> {

		// Figure out batch size.
		let mut available_size_per_source:Vec<usize> = Vec::new();
		if let Some(generator) = &self.generator {
			available_size_per_source.push(generator.amount_available());
		}
		for (connection, cursor) in &self.connections {
			if !channel_is_idle[connection.index] {
				available_size_per_source.push(patcher_buffers[connection.index].currently_stored(cursor));
			}
		}
		if available_size_per_source.is_empty() {
			return Vec::new();
		}
		let batch_size:usize = available_size_per_source.clone().into_iter().min().unwrap_or_default().min(max_batch_size);

		// Get a buffer for each audio source.
		let mut source_buffers:Vec<Vec<f32>> = Vec::with_capacity(self.connections.len() + 1);
		if let Some(generator) = &mut self.generator {
			if let Some(generator_data) = generator.take(batch_size) {
				source_buffers.push(generator_data);
			}
		}
		for (connection, cursor) in &self.connections {
			let connection_buffer:&mut Box<CircularBufferMultiReadDyn<f32>> = &mut patcher_buffers[connection.index];
			if connection_buffer.currently_stored(cursor) >= batch_size {
				source_buffers.push(connection_buffer.take(batch_size, cursor));
			}
		}
		if source_buffers.is_empty() {
			return Vec::new();
		}

		// Combine and return collected buffers.
		let mut combined_buffer:Vec<f32> = source_buffers.remove(source_buffers.len() - 1);
		for additional_buffer in source_buffers {
			for index in 0..additional_buffer.len() {
				combined_buffer[index] += additional_buffer[index];
			}
		}
		combined_buffer
	}
}