use std::{error::Error, time::{ Duration, Instant } };
use minifb::{ Window, WindowOptions };
use grid_kit::Grid;



const DISPLAY_WIDTH:usize = 800;
const DISPLAY_HEIGHT:usize = 200;
const CHANNEL_PADDING:usize = 1;
const UPDATE_INTERVAL:Duration = Duration::from_millis(1000 / 30);
const PEAK_DECAY_PER_SECOND:f32 = 2.0;



pub struct PatcherDisplay {
	window:Window,
	last_display_update:Instant,
	last_peak_update:Instant,
	peaks:Vec<f32>
}
impl PatcherDisplay {

	/* CONSTRUCTOR METHODS */

	/// Create a new display.
	pub fn new() -> Result<PatcherDisplay, Box<dyn Error>> {
		let update_instant:Instant = Instant::now() - UPDATE_INTERVAL;
		Ok(
			PatcherDisplay {
				window: Window::new("audio_patcher_display", DISPLAY_WIDTH, DISPLAY_HEIGHT, WindowOptions::default())?,
				last_display_update: update_instant,
				last_peak_update: update_instant,
				peaks: Vec::new()
			}
		)
	}



	/* PROPERTY GETTER METHODS */

	/// Wether or not the window is open.
	pub fn is_open(&self) -> bool {
		self.window.is_open()
	}



	/* USAGE METHODS */

	/// Update the display.
	pub fn update(&mut self, channel_peaks:Vec<f32>) -> Result<(), Box<dyn Error>> {
		let now:Instant = Instant::now();
		self.update_peaks(channel_peaks, now)?;
		if self.is_open() {
			self.update_display(now)?;
		}
		Ok(())
	}

	/// Update the display.
	fn update_peaks(&mut self, channel_peaks:Vec<f32>, now:Instant) -> Result<(), Box<dyn Error>> {

		// Figure out decay for passed duration.
		let peak_elapsed:Duration = now.duration_since(self.last_peak_update);
		self.last_peak_update = now;
		let max_peak_decay:f32 = PEAK_DECAY_PER_SECOND * peak_elapsed.as_secs_f32();

		// Make sure own peak count matches channel peak count.
		while self.peaks.len() < channel_peaks.len() {
			self.peaks.push(0.0);
		}
		while self.peaks.len() > channel_peaks.len() {
			self.peaks.remove(self.peaks.len() - 1);
		}

		// Set or decay all peaks.
		for index in 0..self.peaks.len() {
			let target:f32 = if index < channel_peaks.len() { channel_peaks[index] } else { 0.0 };
			if target > self.peaks[index] {
				self.peaks[index] = target;
			} else {
				let decay:f32 = (target - self.peaks[index]).max(max_peak_decay);
				self.peaks[index] = (self.peaks[index] - decay).max(0.0);
			}
		}

		// Return success.
		Ok(())
	}

	/// Update the display.
	fn update_display(&mut self, now:Instant) -> Result<(), Box<dyn Error>> {

		// Adhere to display interval.
		if now.duration_since(self.last_display_update) < UPDATE_INTERVAL {
			return Ok(());
		}
		self.last_display_update = now;

		// Create buffer.
		let channel_width:usize = DISPLAY_WIDTH / self.peaks.len();
		let channel_inner_height:usize = DISPLAY_HEIGHT - CHANNEL_PADDING * 2;
		let mut buffer:Grid<u32> = Grid::new(vec![0xFF000000; DISPLAY_WIDTH * DISPLAY_HEIGHT], DISPLAY_WIDTH, DISPLAY_HEIGHT);
		for y in CHANNEL_PADDING..DISPLAY_HEIGHT - CHANNEL_PADDING {
			for channel_index in 0..self.peaks.len() {
				let start_x:usize = channel_index * channel_width + CHANNEL_PADDING;
				let end_x:usize = (channel_index + 1) * channel_width - CHANNEL_PADDING;
				let channel_peak_y:usize = DISPLAY_HEIGHT - (self.peaks[channel_index].abs().min(1.0) * channel_inner_height as f32) as usize;
				for x in start_x..end_x {
					buffer[(x, y)] = if y <= channel_peak_y { 0xFF110E15 } else { 0xFFFF0000 };
				}
			}
		}

		// Apply buffer to window.
		self.window.update_with_buffer(buffer.data(), DISPLAY_WIDTH, DISPLAY_HEIGHT)?;

		// Return success.
		Ok(())
	}
}