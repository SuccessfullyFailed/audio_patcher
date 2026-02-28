use crate::audio_effect::{ AudioEffect, AudioEffectSetting };



const MULTIPLIER_SETTING_NAME:&str = "volume_multiplier";



pub struct VolumeAmplifier {
	settings:Vec<AudioEffectSetting>
}
impl VolumeAmplifier {

	/// Create a new volume amplifier.
	pub fn new(volume_multiplier:f32) -> VolumeAmplifier {
		VolumeAmplifier {
			settings: vec![
				AudioEffectSetting::new(MULTIPLIER_SETTING_NAME, volume_multiplier)
			]
		}
	}
}
impl AudioEffect for VolumeAmplifier {
	fn apply(&mut self, buffer:&mut [f32]) {
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