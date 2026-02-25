use mini_ini_parser::Ini;
use std::error::Error;
use file_ref::FileRef;



/// Read the settings from the settings ini file.
pub fn read_settings() -> Result<Ini, Box<dyn Error>> {
	const SETTINGS_FILE:FileRef = FileRef::new_const("settings.ini");
	const INI_ENCODER:fn(&str) -> String = |value:&str| value.to_string();
	const INI_DECODER:fn(&str) -> String = |value:&str| value.to_string();

	Ini::from_file(SETTINGS_FILE.path(), &INI_ENCODER, &INI_DECODER)
}