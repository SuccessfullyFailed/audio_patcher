use crate::audio_effect::AudioEffect;



const DEFAULT_MULTIPLIER:f32 = 1.0;



pub struct VolumeAmplifier {
	volume_multiplier:f32
}
impl VolumeAmplifier {
	pub const NAME:&str = "volume_amplifier";
}
impl AudioEffect for VolumeAmplifier {
	fn apply_to_buffer(&mut self, buffer:&mut [f32]) {
		if self.volume_multiplier != 1.0 {
			buffer.iter_mut().for_each(|sample| *sample *= self.volume_multiplier);
		}
	}

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