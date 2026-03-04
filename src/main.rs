use crate::{ patcher::Patcher, settings::read_settings };
use std::{ error::Error, time::{ Duration } };
use mini_ini_parser::Ini;



mod settings;
mod patcher;
mod patcher_channel;
mod device;
mod audio_effect;
mod audio_effects;



const SAMPLE_RATE:u32 = 48_000;
const BUFFER_SIZE:usize = SAMPLE_RATE as usize / 10;
const BATCHES_PER_SECOND:u32 = 100;
const BATCH_SIZE:usize = SAMPLE_RATE as usize / BATCHES_PER_SECOND as usize;
const MAX_PATCHER_CHANNELS:usize = 32;
const MAX_CONNECTIONS_PER_NODE:usize = MAX_PATCHER_CHANNELS - 1;



fn main() -> Result<(), Box<dyn Error>> {

	// Read settings.
	let settings:Ini = read_settings()?;

	// Build patcher.
	let mut patcher:Patcher<MAX_PATCHER_CHANNELS, SAMPLE_RATE, BUFFER_SIZE, MAX_CONNECTIONS_PER_NODE> = Patcher::new();
	patcher.update_from_settings(&settings)?;
	patcher.start_streams()?;
	patcher.run(BATCH_SIZE, Duration::from_millis(1))?;

	// Return success.
	Ok(())
}