use crate::{ audio_effect::{AudioEffect, AudioEffectPlaceHolder}, device::{ InputDevice, OutputDevice } };
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
	connections:Vec<(PatcherChannelId, ReadCursor)>,
	input_device:Option<InputDevice<SAMPLE_RATE, BUFFER_SIZE>>,
	effects:Vec<Box<dyn AudioEffect>>,
	output_device:Option<OutputDevice<SAMPLE_RATE, BUFFER_SIZE>>
}
impl<const SAMPLE_RATE:u32, const BUFFER_SIZE:usize> PatcherChannel<SAMPLE_RATE, BUFFER_SIZE> {

	/* CONSTRUCTOR METHODS */

	/// Create a new channel.
	pub fn new(channel_index:usize, channel_name:&str) -> Self {
		PatcherChannel {
			id: PatcherChannelId::new(channel_index, channel_name),
			connections: Vec::new(),
			input_device: None,
			effects: Vec::new(),
			output_device: None
		}
	}

	/// Set an input device.
	pub fn set_input_device(&mut self, input_device:InputDevice<SAMPLE_RATE, BUFFER_SIZE>) {
		self.input_device = Some(input_device);
	}

	/// Set an output device.
	pub fn set_output_device(&mut self, output_device:OutputDevice<SAMPLE_RATE, BUFFER_SIZE>) {
		self.output_device = Some(output_device);
	}

	/// Add a connection to another channel.
	pub fn add_connection(&mut self, channel_id:&PatcherChannelId, patcher_buffer_cursor:ReadCursor) -> Result<(), Box<dyn Error>> {
		if channel_id.index < self.id.index {
			Err(format!("Cannot create connection from channel {} to channel {}, can only create connections with higher indexes.", self.id.index, channel_id.index).into())
		} else {
			self.connections.push((channel_id.clone(), patcher_buffer_cursor));
			Ok(())
		}
	}

	/// Add an effect to the list.
	pub fn add_effect<Effect:AudioEffect + 'static>(&mut self, effect:Effect) {
		self.effects.push(Box::new(effect));
	}

	/// Set an effect to a specific slot.
	pub fn set_effect_to_slot<Effect:AudioEffect + 'static>(&mut self, slot_index:usize, effect:Effect) {
		if self.effects.len() <= slot_index {
			for _ in self.effects.len()..slot_index {
				self.effects.push(Box::new(AudioEffectPlaceHolder::new()));
			}
		}
		self.add_effect(effect);
	}



	/* PROPERTY GETTER METHODS */

	/// Get the ID of the channel.
	pub fn id(&self) -> &PatcherChannelId {
		&self.id
	}

	/// Wether or not this channel is doing anything. Returns true if the channel does not alter or generate any audio.
	pub fn is_idle(&self) -> bool {
		self.input_device.is_none() && self.output_device.is_none() && self.effects.is_empty()
	}

	/// Get a mutable reference to the input device of this channel. Returns None if no device is set.
	pub fn input_device_mut(&mut self) -> &mut Option<InputDevice<SAMPLE_RATE, BUFFER_SIZE>> {
		&mut self.input_device
	}

	/// Get a mutable reference to the output device of this channel. Returns None if no device is set.
	pub fn output_device_mut(&mut self) -> &mut Option<OutputDevice<SAMPLE_RATE, BUFFER_SIZE>> {
		&mut self.output_device
	}

	/// Get a mutable reference to the effects on this channel.
	pub fn effects_mut(&mut self) -> &mut Vec<Box<dyn AudioEffect>> {
		&mut self.effects
	}



	/* USAGE METHODS */

	/// Create a buffer from this channels' input device and connections combined.
	pub fn get_combined_input_buffer(&mut self, patcher_buffers:&mut [Box<CircularBufferMultiReadDyn<f32>>], batch_size:usize) -> Vec<f32> {

		// Get buffer from input device and connected channels.
		let input_device_buffer:Option<Vec<f32>> = match self.input_device_mut() { Some(device) => device.take_from_buffer(batch_size), None => None };
		let mut connection_buffers:Vec<Vec<f32>> = Vec::new();
		for (connection, buffer_cursor) in &self.connections {
			let buffer:&mut CircularBufferMultiReadDyn<f32> = &mut patcher_buffers[connection.index];
			if buffer.currently_stored(buffer_cursor) >= batch_size {
				connection_buffers.push(buffer.take(batch_size, buffer_cursor));
			}
		}

		// Return combined buffers.
		if connection_buffers.is_empty() {
			input_device_buffer.unwrap_or_default()
		} else {
			let mut combined_buffer:Vec<f32> = input_device_buffer.unwrap_or(connection_buffers.remove(connection_buffers.len() - 1));
			let longest_buffer_len:usize = combined_buffer.len().max(connection_buffers.iter().map(|buffer| buffer.len()).max().unwrap_or_default());
			combined_buffer.extend(vec![0.0; longest_buffer_len - combined_buffer.len()]);

			// Looping through sample per index, then buffer index feels logical, but looping through entire buffers is more favorable with CPU cache.
			for additional_buffer in connection_buffers {
				for index in 0..additional_buffer.len() {
					combined_buffer[index] += additional_buffer[index];
				}
			}

			// Normalize sample, make sure the buffers don't exceed max volume.
			for sample in &mut combined_buffer {
				if *sample > 1.0 {
					*sample = 1.0;
				} else if *sample < -1.0 {
					*sample = -1.0;
				}
			}

			combined_buffer
		}
	}
}