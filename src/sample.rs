

pub struct Sample {
    sample_data: Vec<f32>,

    position: Option<usize>,
    real_sample_length: usize,
    max_block_length: usize

}

impl Sample {
    pub fn new(mut sample_data: Vec<f32>, samplerate: f32, max_block_length: usize) -> Self {
	let real_sample_length = sample_data.len();
	let frames = real_sample_length / 2;

	let reserve_frames = ((frames / max_block_length) + 2) * max_block_length;
	sample_data.resize(reserve_frames * 2, 0.0);

	Sample {
	    sample_data: sample_data,

	    position: None,
	    real_sample_length: real_sample_length,
	    max_block_length: max_block_length
	}
    }

    pub fn is_playing(&self) -> bool {
	self.position.is_some()
    }

    pub fn note_on(&mut self) {
	self.position = Some(0);
    }

    pub fn note_off(&mut self) {
	self.position = None;
    }

    pub fn iter(&mut self) -> SampleIterator {
	let pos = self.position.unwrap();
	SampleIterator {
	    sample: self,
	    pos: pos
	}
    }

    fn update(&mut self, new_pos: usize) {
	self.position = if new_pos < self.real_sample_length {
	    Some(new_pos)
	} else {
	    None
	}
    }
}

pub struct SampleIterator<'a> {
    sample: &'a mut Sample,
    pos: usize
}

impl<'a> Iterator for SampleIterator<'a> {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
	let left = self.sample.sample_data[self.pos];
	let right = self.sample.sample_data[self.pos+1];

	self.pos += 2;

	Some((left, right))
    }
}

impl<'a> Drop for SampleIterator<'a> {
    fn drop(&mut self) {
	self.sample.update(self.pos)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn sample_data_length() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let mut sample = Sample::new(sample, 1.0, 16);
	assert_eq!(sample.sample_data.len(), 64);
    }
}
