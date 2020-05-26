use std::f32::consts::PI;

pub struct Sample {
    sample_data: Vec<f32>,

    position: Option<usize>,
    real_sample_length: usize,
    max_block_length: usize,

    native_frequency: f32,
}

impl Sample {
    pub fn new(mut sample_data: Vec<f32>, samplerate: f32, max_block_length: usize, native_frequency: f32) -> Self {
	let real_sample_length = sample_data.len();
	let frames = real_sample_length / 2;

	let reserve_frames = ((frames / max_block_length) + 2) * max_block_length;
	sample_data.resize(reserve_frames * 2, 0.0);

	Sample {
	    sample_data: sample_data,

	    position: None,
	    real_sample_length: frames,
	    max_block_length: max_block_length,

	    native_frequency: native_frequency
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

    pub fn iter(&mut self, frequency: f32) -> SampleIterator {
	let pos = self.position.unwrap();
	let ratio = frequency / self.native_frequency;
	SampleIterator {
	    sample: self,
	    pos: pos,

	    freq_ratio: ratio
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
    pos: usize,

    freq_ratio: f32
}

fn cubic(sample_data: &[f32], pos: usize, remainder: f32) -> f32 {
    let len = sample_data.len();
//    println!("pos {} {} {} {}", ((pos + len) - 2) % len, pos, pos+2, pos+4);
    let p0 = sample_data[((pos + len) - 2) % len];
    let p1 = sample_data[pos];
    let p2 = sample_data[pos+2];
    let p3 = sample_data[pos+4];

    let a = remainder;
    let b = 1.0 - a;
    let c = a * b;

    let r = (1.0 + 1.5 * c) * (p1 * b + p2 * a) - 0.5 * c * (p0 * b + p1 + p2 + p3 * a);

//    println!("res {} {} {} {} : {} -> {}", p0, p1, p2, p3, remainder, r);
    r
}



impl<'a> Iterator for SampleIterator<'a> {
    type Item = (f32, f32);


    fn next(&mut self) -> Option<Self::Item> {

	let interpos = (self.pos as f32) * self.freq_ratio;
	let remainder = interpos - interpos.floor();
	let new_pos = interpos as usize;

//	println!("{}: {} {} -> {} {} {}", self.pos, interpos, remainder, new_pos, 2*new_pos, 2*new_pos+1);

	let left = cubic(&self.sample.sample_data, 2*new_pos, remainder);
	let right = cubic(&self.sample.sample_data, 2*new_pos+1, remainder);

	self.pos += 1;

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
    fn test_iterator() {
	let v = vec![0.0, 0.0,
		     1.0, -1.0,
		     2.0, -2.0,
		     3.0, -3.0,
		     4.0, -4.0,
		     0.0, 0.0];
	let mut sample = Sample::new(v, 1.0, 8, 1.0);
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

	let sample = Sample::new(sample, 1.0, 16, 440.0);
	assert_eq!(sample.sample_data.len(), 64);
    }


    fn make_test_sample(nsamples: usize, samplerate: f32, freq: f32) -> Sample {
	let omega = freq/samplerate * 2.0*PI;
	let sample_data = (0..nsamples*2).map(|t| ((omega * (t/2) as f32).sin())).collect();

	Sample::new(sample_data, samplerate, 4096, freq)
    }


    fn assert_frequency(mut sample: Sample, samplerate: f32, test_freq: f32) {
	let mut i = 0;
	let mut halfw_l = 0.0;
	let mut halfw_r = 0.0;
	let mut last_l = 0.0;
	let mut last_r = 0.0;

	let length = sample.real_sample_length/2;

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
	    if i >= length {
		break;
	    }
	}

	let to_freq = samplerate/((sample.real_sample_length/2) as f32);

	if  halfw_l * to_freq > test_freq || (halfw_l + 1.0) * to_freq < test_freq {
	    panic!("left frequency does not match {} {} {}", halfw_l * to_freq, (halfw_l + 1.0) * to_freq, test_freq)
	}
	if  halfw_r * to_freq > test_freq || (halfw_r + 1.0) * to_freq < test_freq {
	    panic!("left frequency does not match {} {} {}", halfw_r * to_freq, (halfw_r + 1.0) * to_freq, test_freq)
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
