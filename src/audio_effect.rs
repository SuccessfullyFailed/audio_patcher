use std::error::Error;



pub trait AudioEffect {
	
	/// Apply the effect to a buffer.
	fn apply_to_buffer(&mut self, buffer:&mut [f32]);

	/// Get the list of settings for this effect.
	fn settings(&self) -> &[AudioEffectSetting];

	/// Get a mutable reference to the list of settings.
	fn settings_mut(&mut self) -> &mut Vec<AudioEffectSetting>;



	/// Find the value of a setting.
	fn get_setting(&self, name:&str) -> Option<f32> {
		self.settings().into_iter().find(|setting| setting.name == name).map(|setting| setting.value)
	}

	/// Find the value of a setting or return the default value. Does the same as `get_setting().unwrap_or(default_value)`, but shorter.
	fn get_setting_or(&self, name:&str, default_value:f32) -> f32 {
		self.get_setting(name).unwrap_or(default_value)
	}

	/// Set the value of a setting. Creates the setting if it does not exist yet.
	fn set_setting(&mut self, name:&str, value:f32) {
		match self.settings_mut().iter_mut().find(|setting| setting.name == name) {
			Some(setting) => setting.value = value,
			None => self.settings_mut().push(AudioEffectSetting::new(name, value))
		}
	}
}
pub trait SizedAudioEffect:AudioEffect + Sized + Default {

	/// Create the effect from a settings string.
	fn from_settings_str(settings_str:&str) -> Result<Self, Box<dyn Error>> {
		let mut effect = Self::default();
		for setting_str in settings_str.split(',').map(|value| value.trim()).filter(|value| !value.is_empty()) {
			match &setting_str.split(':').collect::<Vec<&str>>()[..] {
				[setting_name] => effect.set_setting(setting_name, 1.0),
				[setting_name, setting_value] => effect.set_setting(setting_name, setting_value.parse::<f32>()?),
				_ => {}
			}
		}
		Ok(effect)
	}
}
impl<T:AudioEffect + Sized + Default> SizedAudioEffect for T {}




pub struct AudioEffectSetting {
	name:String,
	value:f32
}
impl AudioEffectSetting {

	/// Create a new setting.
	pub fn new(name:&str, value:f32) -> AudioEffectSetting {
		AudioEffectSetting {
			name: name.to_string(),
			value
		}
	}
}