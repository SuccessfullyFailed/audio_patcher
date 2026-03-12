pub trait AudioEffect {

	/// Initialize the effect. Is optional.
	fn initialize(&mut self, _sample_rate:u32) {}

	/// Wether or not this generator is currently affecting audio.
	/// Returns true when no audio is being modified.
	fn is_idle(&self) -> bool;

	/// Apply the effect to a buffer.
	fn apply_to_buffer(&mut self, buffer:&mut [f32]);

	/// Set the value of a setting. Does nothing if the effect does not exist.
	fn set_setting(&mut self, name:&str, value:&str);
}
impl AudioEffect for Box<dyn AudioEffect> {

	/// Initialize the effect. Is optional.
	fn initialize(&mut self, sample_rate:u32) {
		let self_unwrapped:&mut dyn AudioEffect = &mut **self;
		self_unwrapped.initialize(sample_rate);
	}

	/// Wether or not this generator is currently affecting audio.
	/// Returns true when no audio is being modified.
	fn is_idle(&self) -> bool {
		let self_unwrapped:&dyn AudioEffect = &**self;
		self_unwrapped.is_idle()
	}

	/// Apply the effect to a buffer.
	fn apply_to_buffer(&mut self, buffer:&mut [f32]) {
		let self_unwrapped:&mut dyn AudioEffect = &mut **self;
		self_unwrapped.apply_to_buffer(buffer);
	}

	/// Set the value of a setting. Does nothing if the setting does not exist.
	fn set_setting(&mut self, name:&str, value:&str) {
		let self_unwrapped:&mut dyn AudioEffect = &mut **self;
		self_unwrapped.set_setting(name, value);
	}
}



pub trait SizedAudioEffect:AudioEffect + Sized + Default {

	/// Return self with a list of settings applied.
	fn with_settings(mut self, settings:&[(&str, &str)]) -> Self {
		for (name, value) in settings {
			self.set_setting(name, value);
		}
		self
	}
}
impl<T:AudioEffect + Sized + Default> SizedAudioEffect for T {}



pub struct AudioEffectPlaceHolder {
}
impl AudioEffectPlaceHolder {
	pub const fn new() -> AudioEffectPlaceHolder {
		AudioEffectPlaceHolder {
		}
	}
}
impl AudioEffect for AudioEffectPlaceHolder {
	fn apply_to_buffer(&mut self, _buffer:&mut [f32]) {}
	fn is_idle(&self) -> bool { true }
	fn set_setting(&mut self, _name:&str, _value:&str) {}
}