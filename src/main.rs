use crate::{ patcher::Patcher, settings::read_settings };
use std::{ error::Error, time::Duration };
use mini_ini_parser::Ini;



mod settings;
mod patcher;
mod patcher_channel;
mod device;
mod audio_effect;
mod audio_effects;
mod display;



const SAMPLE_RATE:u32 = 48_000;
const BUFFER_SIZE:usize = SAMPLE_RATE as usize;
const UPDATE_INTERVAL:Duration = Duration::from_millis(1);



static mut PATCHER:Patcher<SAMPLE_RATE, BUFFER_SIZE> = Patcher::new();




#[allow(static_mut_refs)]
fn main() -> Result<(), Box<dyn Error>> {

	// Read settings.
	let settings:Ini = read_settings()?;

	unsafe {
		PATCHER = Patcher::new();
	}
	// Build patcher.
	let mut patcher:Patcher<SAMPLE_RATE, BUFFER_SIZE> = Patcher::new();
	patcher.add_display()?;
	patcher.update_from_settings(&settings)?;
	patcher.run(UPDATE_INTERVAL)?;

	// Return success.
	Ok(())
}