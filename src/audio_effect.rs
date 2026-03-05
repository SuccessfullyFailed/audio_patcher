use std::error::Error;



pub trait AudioEffect {
	
	/// Get the name of the effect.
	fn name(&self) -> &str;

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

	/// Return self with a list of settings applied.
	fn with_settings(mut self, settings:&[(String, f32)]) -> Self {
		for (name, value) in settings {
			self.set_setting(name, *value);
		}
		self
	}
}
impl<T:AudioEffect + Sized + Default> SizedAudioEffect for T {}



pub struct AudioEffectPlaceHolder {
	settings:Vec<AudioEffectSetting>
}
impl AudioEffectPlaceHolder {
	pub const fn new() -> AudioEffectPlaceHolder {
		AudioEffectPlaceHolder {
			settings: Vec::new()
		}
	}
}
impl AudioEffect for AudioEffectPlaceHolder {
	fn name(&self) -> &str { "" }
	fn apply_to_buffer(&mut self, _buffer:&mut [f32]) {}
	fn settings(&self) -> &[AudioEffectSetting] { &self.settings }
	fn settings_mut(&mut self) -> &mut Vec<AudioEffectSetting> { &mut self.settings }
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

	/// Get the name of the setting.
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get the value of the settings.
	pub fn value(&self) -> f32 {
		self.value
	}
}