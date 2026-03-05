use crate::audio_effect::{ AudioEffect, AudioEffectSetting };



const MULTIPLIER_SETTING_NAME:&str = "volume_multiplier";
const DEFAULT_MULTIPLIER:f32 = 1.0;



pub struct VolumeAmplifier {
	settings:Vec<AudioEffectSetting>
}
impl VolumeAmplifier {
	pub const NAME:&str = "volume_amplifier";
}
impl AudioEffect for VolumeAmplifier {
	fn name(&self) -> &str {
		Self::NAME
	}
	fn apply_to_buffer(&mut self, buffer:&mut [f32]) {
		if let Some(multiplier) = self.get_setting(MULTIPLIER_SETTING_NAME) {
			if multiplier != 1.0 {
				buffer.iter_mut().for_each(|sample| *sample *= multiplier);
			}
		}
	}
	fn settings(&self) -> &[AudioEffectSetting] {
		&self.settings
	}
	fn settings_mut(&mut self) -> &mut Vec<AudioEffectSetting> {
		&mut self.settings
	}
}
impl Default for VolumeAmplifier {
	fn default() -> Self {
		VolumeAmplifier {
			settings: vec![
				AudioEffectSetting::new(MULTIPLIER_SETTING_NAME, DEFAULT_MULTIPLIER)
			]
		}
	}
}