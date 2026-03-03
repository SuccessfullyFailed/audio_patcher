use crate::{ settings::read_settings, audio_effect::SizedAudioEffect, audio_effects::VolumeAmplifier, device::{ InputDevice, OutputDevice }, patcher_channel::{ PatcherChannel, PatcherChannelId } };
use std::{ error::Error, thread::sleep, time::{ Duration, Instant } };
use circular_buffer::{ CircularBuffer, CircularBufferMultiRead };
use mini_ini_parser::{ Ini, IniCategory };



mod settings;
mod patcher_channel;
mod device;
mod audio_effect;
mod audio_effects;



pub const SAMPLE_RATE:u32 = 48_000;
pub const BUFFER_SIZE:usize = SAMPLE_RATE as usize;
pub const BATCHES_PER_SECOND:u32 = 100;
pub const BATCH_SIZE:usize = SAMPLE_RATE as usize / BATCHES_PER_SECOND as usize;

const MAX_PATCHER_CHANNELS:usize = 32;
static mut PATCHER_CHANNELS:[Option<PatcherChannel>; MAX_PATCHER_CHANNELS] = [const { None }; MAX_PATCHER_CHANNELS];
static mut PATCHER_INPUT_BUFFERS:[CircularBuffer<f32, BUFFER_SIZE>; MAX_PATCHER_CHANNELS] = [CircularBuffer::new_const(0.0); MAX_PATCHER_CHANNELS];
static mut PATCHER_OUTPUT_BUFFERS:[CircularBufferMultiRead<f32, BUFFER_SIZE, MAX_PATCHER_CHANNELS>; MAX_PATCHER_CHANNELS] = [CircularBufferMultiRead::new_const(0.0); MAX_PATCHER_CHANNELS];



fn main() -> Result<(), Box<dyn Error>> {

	// Read settings.
	let settings:Ini = read_settings()?;

	// Build patcher channels.
	// Build from right to left, making sure connection sources are created before their targets.
	for channel_index in (0..MAX_PATCHER_CHANNELS).rev() {
		let channel_settings:&IniCategory = &settings[&format!("channel_{channel_index}")];
		if channel_settings.is_ok() {
			unsafe {
				let channel_name:&str = &channel_settings["name"].value;
				let mut channel:PatcherChannel = PatcherChannel::new(channel_index, channel_name);

				// Handle input and output device.
				if channel_settings["input_device"].is_ok() {
					if let Some(input_device) = InputDevice::new(&channel_settings["input_device"].value)? {
						channel.set_input_device(input_device);
					}
				}
				if channel_settings["output_device"].is_ok() {
					if let Some(output_device) = OutputDevice::new(&channel_settings["output_device"].value)? {
						channel.set_output_device(output_device);
					}
				}

				// Handle connections to other channels.
				if channel_settings["connections"].is_ok() {
					for connection_channel_name in channel_settings["connections"].value.split(", ").map(|name| name.trim()).filter(|name| !name.is_empty()) {
						let mut valid_connection:bool = false;
						#[allow(static_mut_refs)]
						if let Some(connection_channel_index) = PATCHER_CHANNELS.iter().skip(channel_index + 1).position(|channel| channel.as_ref().is_some_and(|channel| channel.id().name == connection_channel_name)).map(|offset| channel_index + 1 + offset) {
							let connection_id:PatcherChannelId = PATCHER_CHANNELS[connection_channel_index].as_ref().unwrap().id().clone();
							if let Err(error) = channel.add_connection(&connection_id) {
								eprintln!("Could not create connection: {error}");
							} else {
								valid_connection = true;
							}
						}
						if !valid_connection {
							eprintln!("Could not create connection from '{channel_name}' to '{connection_channel_name}'.");
						}
					}
				}

				// Handle effects.
				if channel_settings[VolumeAmplifier::NAME].is_ok() {
					channel.add_effect(VolumeAmplifier::from_settings_str(&channel_settings[VolumeAmplifier::NAME].value)?);
				}

				PATCHER_CHANNELS[channel_index] = Some(channel);
			}
		}
	}

	// Create streams for devices from right to left.
	for patcher_channel_index in (0..MAX_PATCHER_CHANNELS).rev() {
		unsafe {
			if let Some(channel) = &mut PATCHER_CHANNELS[patcher_channel_index] {
				let channel_id:PatcherChannelId = channel.id().clone();
				if let Some(input_device) = channel.input_device_mut() {
					input_device.create_stream(&channel_id)?;
				}
				if let Some(output_device) = channel.output_device_mut() {
					output_device.create_stream(&channel_id, PATCHER_OUTPUT_BUFFERS[channel_id.index].create_read_cursor())?;
				}
			}
		}
	}

	// Keep moving data from buffers to their targets.
	const INTERVAL_DELAY:Duration = Duration::from_millis(1);
	let mut last_interval:Instant = Instant::now() - INTERVAL_DELAY;
	loop {
		// Adhere to interval.
		let now:Instant = Instant::now();
		let duration_since_last_interval:Duration = now.duration_since(last_interval);
		if duration_since_last_interval < INTERVAL_DELAY {
			sleep(INTERVAL_DELAY - duration_since_last_interval);
		}
		last_interval = now;

		// Update buffers from right to left.
		for patcher_channel_index in (0..MAX_PATCHER_CHANNELS).rev() {
			unsafe {
				if let Some(channel) = &mut PATCHER_CHANNELS[patcher_channel_index] {
					let mut channel_buffer:Vec<f32> = channel.get_input_buffer();
					for effect in channel.effects_mut() {
						effect.apply_to_buffer(&mut channel_buffer);
					}
					PATCHER_OUTPUT_BUFFERS[channel.id().index].extend(&channel_buffer);
				}
			}
		}
	}
}