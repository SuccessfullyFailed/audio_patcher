


#[derive(Clone, Copy)]
pub struct ReverseDrawCircularBuffer<T, const CAPACITY:usize> {
	data:[T; CAPACITY],
	write_cursor:usize,
	len:usize
}
impl<T:Copy, const CAPACITY:usize> ReverseDrawCircularBuffer<T, CAPACITY> {

	/// Create a new buffer.
	pub const fn new(default_value:T) -> Self {
		Self {
			data: [default_value; CAPACITY],
			write_cursor: 0,
			len: 0
		}
	}

	/// Extend the data of the buffer.
	pub fn extend(&mut self, data:&[T]) {
		let data_len:usize = data.len();

		let capacity_without_wrap:usize = CAPACITY - self.write_cursor;
		if capacity_without_wrap >= data_len {
			self.data[self.write_cursor..self.write_cursor + data_len].copy_from_slice(&data);
			self.write_cursor += data_len;
			if self.write_cursor == CAPACITY {
				self.write_cursor = 0;
			}
			if self.len < CAPACITY {
				self.len += data_len;
			}
		} else {
			self.extend(&data[..capacity_without_wrap]);
			self.extend(&data[capacity_without_wrap..]);
		}
	}

	/// Take some data from the buffer. Takes as much as possible, but is not guaranteed to have the required amount.
	pub fn take(&self, amount:usize) -> Vec<T> {
		if self.write_cursor >= amount {
			self.data[self.write_cursor - amount..self.write_cursor].to_vec()
		} else if self.len >= amount {
			let mut output:Vec<T> = Vec::with_capacity(amount);
			output.extend(&self.data[CAPACITY - (amount - self.write_cursor)..]);
			output.extend(&self.data[..self.write_cursor]);
			output
		} else {
			self.data[..self.write_cursor].to_vec()
		}
	}

	/// The amount of data available.
	pub fn len(&self) -> usize {
		self.len
	}
}



#[cfg(test)]
#[test]
fn reverse_draw_circular_buffer_full_test() {

	// Create and validate initial buffer.
	let mut buffer:ReverseDrawCircularBuffer<f32, 100> = ReverseDrawCircularBuffer::new(0.0);
	assert_eq!(buffer.len(), 0);
	assert_eq!(buffer.write_cursor, 0);
	assert_eq!(&buffer.data, &[0.0; 100]);
	assert_eq!(buffer.take(10), Vec::new());
	assert_eq!(&buffer.data, &[0.0; 100]);

	// Add data and revalidate.
	buffer.extend(&(0..25).map(|index| index as f32).collect::<Vec<f32>>());
	assert_eq!(buffer.len(), 25);
	assert_eq!(buffer.write_cursor, 25);
	assert_eq!(&buffer.data, &(0..100).map(|index| if index < 25 { index as f32 } else { 0.0 }).collect::<Vec<f32>>()[..]);
	assert_eq!(buffer.take(10), (0..10).map(|index| (index + 15) as f32).collect::<Vec<f32>>());
	assert_eq!(&buffer.data, &(0..100).map(|index| if index < 25 { index as f32 } else { 0.0 }).collect::<Vec<f32>>()[..]);

	// Add overflowing data and revalidate.
	buffer.extend(&(25..120).map(|index| index as f32).collect::<Vec<f32>>());
	assert_eq!(buffer.len(), 100);
	assert_eq!(buffer.write_cursor, 20);
	assert_eq!(&buffer.data, &(0..100).map(|index| if index < 20 { (index + 100) as f32 } else { index as f32 }).collect::<Vec<f32>>()[..]);
	assert_eq!(buffer.take(10), (0..10).map(|index| (index + 110) as f32).collect::<Vec<f32>>());
}