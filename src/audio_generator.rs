use std::error::Error;



pub trait AudioGenerator {

	/// Get the name of the generator.
	fn name(&self) -> &str;
	
	/// Start the generator.
	fn start(&mut self) -> Result<(), Box<dyn Error>> {
		Ok(())
	}

	/// Stop the generator.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		Ok(())
	}

	/// Wether or not this generator is currently outputting audio.
	/// Returns true when no audio is being generated.
	fn is_idle(&self) -> bool;

	/// The amount of data currently available from the generator.
	fn amount_available(&self) -> usize;

	/// Try to take an amount of data from the buffer.
	/// Returns None if the buffer does not contain enough data.
	fn take(&self, amount:usize) -> Option<Vec<f32>>;
}