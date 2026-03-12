use crate::audio_effect::AudioEffect;



const DEFAULT_MULTIPLIER:f32 = 1.0;



pub struct VolumeAmplifier {
	volume_multiplier:f32
}
impl VolumeAmplifier {
	pub const NAME:&str = "volume_amplifier";
}
impl AudioEffect for VolumeAmplifier {

	/// Wether or not this generator is currently affecting audio.
	/// Returns true when no audio is being modified.
	fn is_idle(&self) -> bool {
		self.volume_multiplier == 1.0
	}

	/// Apply the effect to a buffer.
	fn apply_to_buffer(&mut self, buffer:&mut [f32]) {
		if self.volume_multiplier != 1.0 {
			buffer.iter_mut().for_each(|sample| *sample *= self.volume_multiplier);
		}
	}

	/// Set the value of a setting. Does nothing if the setting does not exist.
	fn set_setting(&mut self, name:&str, value:&str) {
		if name == "volume_multiplier" {
			if let Ok(volume_multiplier) = value.parse::<f32>() {
				self.volume_multiplier = volume_multiplier;
			}
		}
	}
}
impl Default for VolumeAmplifier {
	fn default() -> Self {
		VolumeAmplifier {
			volume_multiplier: DEFAULT_MULTIPLIER
		}
	}
}