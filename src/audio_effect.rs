



pub trait AudioEffect {

	/// Apply the effect to a buffer.
	fn apply(&mut self, buffer:&mut [f32]);

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