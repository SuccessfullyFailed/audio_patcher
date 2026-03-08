use std::error::Error;



pub trait AudioEndPoint {

	/// Get the name of the output.
	fn name(&self) -> &str;
	
	/// Start the output.
	fn start(&mut self) -> Result<(), Box<dyn Error>> {
		Ok(())
	}

	/// Stop the output.
	fn stop(&mut self) -> Result<(), Box<dyn Error>> {
		Ok(())
	}

	/// Pass additional data to the output.
	fn extend(&self, data:&[f32]);
}