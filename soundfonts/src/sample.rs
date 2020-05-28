
pub struct Sample {
    sample_data: Vec<f32>,

    position: Option<f64>,
    real_sample_length: f64,
    max_block_length: usize,

    native_frequency: f64,
}

impl Sample {
    pub fn new(mut sample_data: Vec<f32>, max_block_length: usize, native_frequency: f64) -> Self {
	let real_sample_length = sample_data.len();
	let frames = real_sample_length / 2;

	let reserve_frames = ((frames / max_block_length) + 2) * max_block_length;
	sample_data.resize(reserve_frames * 2, 0.0);

	Sample {
	    sample_data: sample_data,

	    position: None,
	    real_sample_length: frames as f64,
	    max_block_length: max_block_length,

	    native_frequency: native_frequency
	}
    }

    pub fn is_playing(&self) -> bool {
	self.position.is_some()
    }

    pub fn note_on(&mut self) {
	self.position = Some(0.0);
    }

    pub fn iter(&mut self, frequency: f64) -> SampleIterator {
	let pos = self.position.unwrap();
	let ratio = frequency / self.native_frequency;
	if pos + self.max_block_length as f64 * ratio >= (self.sample_data.len() / 2) as f64 {
	    self.sample_data.resize(self.sample_data.len() + 2 * self.max_block_length, 0.0)
	}
	SampleIterator::new(self, pos, ratio)
    }

    fn update(&mut self, new_pos: f64) {
	self.position = if new_pos < self.real_sample_length {
	    Some(new_pos)
	} else {
	    None
	}
    }
}

pub struct SampleIterator<'a> {
    sample: &'a mut Sample,
    pos: f64,

    freq_ratio: f64
}

impl<'a> SampleIterator<'a> {
    fn new(sample: &'a mut Sample, pos: f64, freq_ratio: f64) -> Self {
	SampleIterator {
	    sample: sample,
	    pos: pos,
	    freq_ratio: freq_ratio
	}
    }
}

fn cubic(sample_data: &[f32], pos: usize, remainder: f64) -> f32 {
    let len = sample_data.len();

    let p0 = sample_data[((pos + len) - 2) % len] as f64;
    let p1 = sample_data[pos] as f64;
    let p2 = sample_data[pos+2] as f64;
    let p3 = sample_data[pos+4] as f64;

    let a = remainder;
    let b = 1.0 - a;
    let c = a * b;

    ((1.0  + 1.5 * c) * (p1 * b + p2 * a) - 0.5 * c * (p0 * b + p1 + p2 + p3 * a)) as f32
}



impl<'a> Iterator for SampleIterator<'a> {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
	let (remainder, sample_pos) = {
	    let sample_pos = self.pos.floor();
	    ((self.pos - sample_pos), sample_pos as usize)
	};

	let left = cubic(&self.sample.sample_data, 2*sample_pos, remainder);
	let right = cubic(&self.sample.sample_data, 2*sample_pos+1, remainder);

	self.pos += self.freq_ratio;

	Some((left, right))
    }
}

impl<'a> Drop for SampleIterator<'a> {
    fn drop(&mut self) {
	self.sample.update(self.pos);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use std::f64::consts::PI;

    #[test]
    fn test_iterator() {
	let v = vec![0.0, 0.0,
		     1.0, -1.0,
		     2.0, -2.0,
		     3.0, -3.0,
		     4.0, -4.0,
		     0.0, 0.0];
	let mut sample = Sample::new(v, 8, 1.0);
	sample.note_on();

	let mut it = sample.iter(1.0);

	assert_eq!(it.next(), Some((0.0, 0.0)));
	assert_eq!(it.next(), Some((1.0, -1.0)));
	assert_eq!(it.next(), Some((2.0, -2.0)));
	assert_eq!(it.next(), Some((3.0, -3.0)));
	assert_eq!(it.next(), Some((4.0, -4.0)));
	assert_eq!(it.next(), Some((0.0, 0.0)));
    }


    #[test]
    fn sample_data_length() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let sample = Sample::new(sample, 16, 440.0);
	assert_eq!(sample.sample_data.len(), 64);
    }


    fn make_test_sample(nsamples: usize, samplerate: f64, freq: f64) -> Sample {
	let omega = freq/samplerate * 2.0*PI;
	let sample_data = (0..nsamples*2).map(|t| ((omega * (t/2) as f64).sin() as f32)).collect();

	Sample::new(sample_data, 4096, freq)
    }


    fn assert_frequency(mut sample: Sample, samplerate: f64, test_freq: f64) {
	let mut i = 0;
	let mut halfw_l = 0.0;
	let mut halfw_r = 0.0;
	let mut last_l = 0.0;
	let mut last_r = 0.0;

	let length = sample.real_sample_length/2.0;

	for (sl, sr) in sample.iter(test_freq) {
	    if sl * last_l < 0.0 {
		halfw_l += 0.5;
	    }
	    if sr * last_r < 0.0 {
		halfw_r += 0.5;
	    }

	    last_l = sl;
	    last_r = sl;

	    i += 1;
	    if i as f64 >= length {
		break;
	    }
	}

	let to_freq = samplerate/((sample.real_sample_length/2.0) as f64);

	if  halfw_l * to_freq > test_freq || (halfw_l + 1.0) * to_freq < test_freq {
	    panic!("left frequency does not match {} {} {}", halfw_l * to_freq, (halfw_l + 1.0) * to_freq, test_freq)
	}
	if  halfw_r * to_freq > test_freq || (halfw_r + 1.0) * to_freq < test_freq {
	    panic!("left frequency does not match {} {} {}", halfw_r * to_freq, (halfw_r + 1.0) * to_freq, test_freq)
	}
    }

    #[test]
    fn test_pitch_up_at_start() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on();

	while sample.is_playing() {
	    let mut it = sample.iter(880.0);
	    for _i in 0..4096 {
		it.next();
	    }
	}
    }

    #[test]
    fn test_pitch_up_late() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on();

	let pitch_freq = 440.0;
	while let Some(pos) = sample.position {
	    let freq = if pos < 30000.0 {
		pitch_freq
	    } else {
		2.0 * pitch_freq
	    };
	    let mut it = sample.iter(freq);
	    for _i in 0..4096 {
		it.next();
	    }
	}
    }

    #[test]
    fn test_test_sample_native() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on();
	assert_frequency(sample, 48000.0, 440.0);
    }

    #[test]
    fn test_test_sample_half_tone_up() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on();
	assert_frequency(sample, 48000.0, 466.16);
    }

    #[test]
    fn test_test_sample_half_tone_down() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on();
	assert_frequency(sample, 48000.0, 415.30);
    }

    #[test]
    fn test_cubic_interpolation() {
	let d = [0.0, 0.0,
		 1.0, -1.0,
		 2.0, -2.0,
		 3.0, -3.0,
		 4.0, -4.0,
		 0.0, 0.0];

	assert_eq!(cubic(&d, 0, 0.0), 0.0);
	assert_eq!(cubic(&d, 2, 0.0), 1.0);
	assert_eq!(cubic(&d, 4, 0.0), 2.0);
	assert_eq!(cubic(&d, 6, 0.0), 3.0);

	assert_eq!(cubic(&d, 1, 0.0), -0.0);
	assert_eq!(cubic(&d, 3, 0.0), -1.0);
	assert_eq!(cubic(&d, 5, 0.0), -2.0);
	assert_eq!(cubic(&d, 7, 0.0), -3.0);

	assert_eq!(cubic(&d, 4, 0.5), 2.5);
	assert_eq!(cubic(&d, 5, 0.5), -2.5);
    }
}
