use crate::{ audio_effect::AudioEffect, device::{ InputDevice, OutputDevice } };
use circular_buffer::{ CircularBuffer, CircularBufferMultiRead, ReadCursor };
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



pub struct PatcherChannel {
	id:PatcherChannelId,
	connections:Vec<(PatcherChannelId, ReadCursor)>,
	input_device:Option<InputDevice>,
	effects:Vec<Box<dyn AudioEffect>>,
	output_device:Option<OutputDevice>
}
impl PatcherChannel {

	/* CONSTRUCTOR METHODS */

	/// Create a new channel.
	pub fn new(channel_index:usize, channel_name:&str) -> PatcherChannel {
		PatcherChannel {
			id: PatcherChannelId::new(channel_index, channel_name),
			connections: Vec::new(),
			input_device: None,
			effects: Vec::new(),
			output_device: None
		}
	}

	/// Set an input device.
	pub fn set_input_device(&mut self, input_device:InputDevice) {
		self.input_device = Some(input_device);
	}

	/// Set an output device.
	pub fn set_output_device(&mut self, output_device:OutputDevice) {
		self.output_device = Some(output_device);
	}

	/// Add a connection to another channel.
	pub fn add_connection<const BUFFER_SIZE:usize, const BUFFER_CURSOR_COUNT:usize>(&mut self, channel_id:&PatcherChannelId, output_buffers:&'static mut [CircularBufferMultiRead<f32, BUFFER_SIZE, BUFFER_CURSOR_COUNT>]) -> Result<(), Box<dyn Error>> {
		if channel_id.index < self.id.index {
			Err(format!("Cannot create connection from channel {} to channel {}, can only create connections with higher indexes.", self.id.index, channel_id.index).into())
		} else {
			self.connections.push((channel_id.clone(), output_buffers[channel_id.index].create_read_cursor()));
			Ok(())
		}
	}

	/// Add an effect to the list.
	pub fn add_effect<Effect:AudioEffect + 'static>(&mut self, effect:Effect) {
		self.effects.push(Box::new(effect));
	}



	/* PROPERTY GETTER METHODS */

	/// Get the ID of the channel.
	pub fn id(&self) -> &PatcherChannelId {
		&self.id
	}

	/// Get a mutable reference to the input device of this channel. Returns None if no device is set.
	pub fn input_device_mut(&mut self) -> &mut Option<InputDevice> {
		&mut self.input_device
	}

	/// Get a mutable reference to the output device of this channel. Returns None if no device is set.
	pub fn output_device_mut(&mut self) -> &mut Option<OutputDevice> {
		&mut self.output_device
	}

	/// Get a mutable reference to the effects on this channel.
	pub fn effects_mut(&mut self) -> &mut Vec<Box<dyn AudioEffect>> {
		&mut self.effects
	}



	/* USAGE METHODS */

	/// Create a buffer from this channels' input device and connections combined.
	pub fn get_combined_input_buffer<const INPUT_BUFFER_SIZE:usize, const OUTPUT_BUFFER_SIZE:usize, const OUTPUT_BUFFER_CURSOR_COUNT:usize>(&mut self, input_device_buffer:&'static mut CircularBuffer<f32, INPUT_BUFFER_SIZE>, output_buffers:&'static mut [CircularBufferMultiRead<f32, OUTPUT_BUFFER_SIZE, OUTPUT_BUFFER_CURSOR_COUNT>], batch_size:usize) -> Vec<f32> {

		// Get buffer from input device and connected channels.
		let input_device_buffer:Option<Vec<f32>> = {
			if input_device_buffer.currently_stored() > batch_size {
				Some(input_device_buffer.take(batch_size))
			} else {
				None
			}
		};
		let mut connection_buffers:Vec<Vec<f32>> = Vec::new();
		for (connection, buffer_cursor) in &self.connections {
			let buffer:&mut CircularBufferMultiRead<f32, OUTPUT_BUFFER_SIZE, OUTPUT_BUFFER_CURSOR_COUNT> = &mut output_buffers[connection.index];
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