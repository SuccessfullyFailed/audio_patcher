use std::{ error::Error, time::Duration };
use audio_patcher::Patcher;
use mini_ini_parser::Ini;
use file_ref::FileRef;



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
	patcher.update_from_ini(&settings)?;
	patcher.run(UPDATE_INTERVAL)?;

	// Return success.
	Ok(())
}



/// Read the settings from the settings ini file.
pub fn read_settings() -> Result<Ini, Box<dyn Error>> {
	const SETTINGS_FILE:FileRef = FileRef::new_const("settings.ini");
	const INI_ENCODER:fn(&str) -> String = |value:&str| value.to_string();
	const INI_DECODER:fn(&str) -> String = |value:&str| value.to_string();

	Ini::from_file(SETTINGS_FILE.path(), &INI_ENCODER, &INI_DECODER)
}