
use super::envelopes;

struct Voice {
    position: f64,
    frequency: f64,
    gain: f32,

    envelope_state: envelopes::State,
    last_envelope_gain: f32,
    release_start_gain: f32
}

impl Voice {
    fn new(frequency: f64, gain: f32) -> Voice {
	Voice {
	    frequency: frequency,
	    gain: gain,
	    position: 0.0,

	    envelope_state: envelopes::State::AttackDecay(0),
	    last_envelope_gain: 1.0,
	    release_start_gain: 1.0
	}
    }
}

pub struct Sample {
    sample_data: Vec<f32>,

    voices: Vec<Voice>,

    real_sample_length: f64,
    max_block_length: usize,

    native_frequency: f64,

    envelope: envelopes::ADSREnvelope,
}

impl Sample {
    pub fn new(mut sample_data: Vec<f32>, max_block_length: usize, native_frequency: f64, envelope: envelopes::ADSREnvelope) -> Self {
	let real_sample_length = sample_data.len();
	let frames = real_sample_length / 2;

	let reserve_frames = ((frames / max_block_length) + 2) * max_block_length;
	sample_data.resize(reserve_frames * 2, 0.0);

	Sample {
	    sample_data: sample_data,

	    voices: Vec::new(),
	    real_sample_length: frames as f64,
	    max_block_length: max_block_length,

	    native_frequency: native_frequency,

	    envelope: envelope
	}
    }

    pub fn is_playing(&self) -> bool {
	!self.voices.is_empty()
    }

    pub fn is_playing_note(&self, frequency: f64) -> bool {
	self.voices.iter().any(|v| v.frequency == frequency)
    }

    pub fn note_on(&mut self, frequency: f64, gain: f32) {
	self.voices.push(Voice::new(frequency, gain))
    }

    pub fn note_off(&mut self, frequency: f64) {
	for voice in &mut self.voices {
	    if (voice.frequency - frequency).abs() < std::f64::EPSILON * frequency {
		voice.envelope_state = envelopes::State::Release(0);
		voice.release_start_gain = voice.last_envelope_gain;
	    }
	}
    }

    pub fn all_notes_off(&mut self) {
	for voice in &mut self.voices {
	    voice.envelope_state = envelopes::State::Release(0);
	    voice.release_start_gain = voice.last_envelope_gain;
	}
    }

    pub fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
	for voice in &mut self.voices {
	    let ratio = voice.frequency / self.native_frequency;
	    if voice.position + self.max_block_length as f64 * ratio >= (self.sample_data.len() / 2) as f64 {
		self.sample_data.resize(self.sample_data.len() + 2 * self.max_block_length, 0.0)
	    }

	    let (envelope, mut env_position) = self.envelope.active_envelope(voice.envelope_state);
	    for (l,r) in Iterator::zip(out_left.iter_mut(), out_right.iter_mut()) {
		let (remainder, sample_pos) = {
		    let sample_pos = voice.position.floor();
		    ((voice.position - sample_pos), sample_pos as usize)
		};
		let gain = voice.gain * envelope[env_position] * voice.release_start_gain;
		*l += gain * cubic(&self.sample_data, 2*sample_pos, remainder);
		*r += gain * cubic(&self.sample_data, 2*sample_pos+1, remainder);
		voice.position += ratio;
		env_position += 1;
	    };
	    voice.last_envelope_gain = *envelope.get(env_position).unwrap_or(&envelope[env_position-1]);
	    self.envelope.update_state(&mut voice.envelope_state, env_position);
	};
	let real_sample_length = self.real_sample_length;
	self.voices.retain(|voice| voice.position < real_sample_length && voice.envelope_state.is_active());
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


#[cfg(test)]
mod tests {

    use super::*;

    use std::f64::consts::PI;
    use std::f32::consts::SQRT_2;

    fn f32_eq(a: f32, b: f32) -> bool {
	if (a - b).abs() > f32::EPSILON {
	    println!("float equivalence check failed, a: {}, b: {}", a, b);
	    false
	} else {
	    true
	}
    }

    /*

    #[test]
    fn test_iterator() {
	let v = vec![0.0, 0.0,
		     1.0, -1.0,
		     2.0, -2.0,
		     3.0, -3.0,
		     4.0, -4.0,
		     0.0, 0.0];
	let mut sample = Sample::new(v, 8, 1.0);
	sample.note_on(1.0, 1.0);

	let mut it = sample.iter(1.0);

	assert_eq!(it.next(), Some((0.0, 0.0)));
	assert_eq!(it.next(), Some((1.0, -1.0)));
	assert_eq!(it.next(), Some((2.0, -2.0)));
	assert_eq!(it.next(), Some((3.0, -3.0)));
	assert_eq!(it.next(), Some((4.0, -4.0)));
	assert_eq!(it.next(), Some((0.0, 0.0)));
    }
    */

    #[test]
    fn sample_data_length() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let sample = Sample::new(sample, 16, 440.0, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, 16));
	assert_eq!(sample.sample_data.len(), 64);
    }


    fn make_test_sample(nsamples: usize, samplerate: f64, freq: f64) -> Sample {
	let omega = freq/samplerate * 2.0*PI;
	let sample_data = (0..nsamples*2).map(|t| ((omega * (t/2) as f64).sin() as f32)).collect();

	Sample::new(sample_data, nsamples, freq, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, nsamples))
    }


    fn assert_frequency(mut sample: Sample, samplerate: f64, test_freq: f64) {
	let mut halfw_l = 0.0;
	let mut halfw_r = 0.0;
	let mut last_l = 0.0;
	let mut last_r = 0.0;

	let length = (sample.real_sample_length/2.0).ceil() as usize;

	let mut out_left = Vec::new();
	out_left.resize(length, 0.0);
	let mut out_right = Vec::new();
	out_right.resize(length, 0.0);

	sample.process(&mut out_left, &mut out_right);

	for (sl, sr) in Iterator::zip(out_left.iter(), out_right.iter()) {
	    if sl * last_l < 0.0 {
		halfw_l += 0.5;
	    }
	    if sr * last_r < 0.0 {
		halfw_r += 0.5;
	    }

	    last_l = *sl;
	    last_r = *sl;
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
    fn test_test_sample_native() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on(440.0, 1.0);
	assert_frequency(sample, 48000.0, 440.0);
    }

    #[test]
    fn test_test_sample_half_tone_up() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on(466.16, 1.0);
	assert_frequency(sample, 48000.0, 466.16);
    }

    #[test]
    fn test_test_sample_half_tone_down() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on(415.30, 1.0);
	assert_frequency(sample, 48000.0, 415.30);
    }

    #[test]
    fn test_pitch_up_at_start() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on(880.0, 1.0);


	while sample.is_playing() {
	    let mut out_left = [0.0; 4096];
	    let mut out_right = [0.0; 4096];
	    sample.process(&mut out_left, &mut out_right);
	}
    }

    #[test]
    fn test_pitch_up_late() {
	let mut sample = make_test_sample(36000, 48000.0, 440.0);
	sample.note_on(440.0, 1.0);

	let pitch_freq = 440.0;
	while sample.is_playing() {
	    let freq = if sample.voices[0].position < 30000.0 {
		pitch_freq
	    } else {
		2.0 * pitch_freq
	    };
	    sample.voices[0].frequency = freq;
	    let mut out_left = [0.0; 4096];
	    let mut out_right = [0.0; 4096];
	    sample.process(&mut out_left, &mut out_right);
	}
    }

    #[test]
    fn simple_sample_process() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let max_block_length = 8;
	let frequency = 1.0;
	let mut sample = Sample::new(sample, max_block_length, frequency, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, max_block_length));

	sample.note_on(frequency, 1.0);

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	sample.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 1.0));
	assert!(f32_eq(out_left[1], 0.5));

	assert!(f32_eq(out_right[0], 0.5));
	assert!(f32_eq(out_right[1], 1.0));

	assert!(sample.is_playing());

	let mut out_left: [f32; 2] = [-0.5, -0.2];
	let mut out_right: [f32; 2] = [-0.2, -0.5];

	sample.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 0.5));
	assert!(f32_eq(out_left[1], -0.2));

	assert!(f32_eq(out_right[0], 0.3));
	assert!(f32_eq(out_right[1], -0.5));

	assert!(!sample.is_playing());
    }

    #[test]
    fn sample_two_notes_process() {
	let sample_data = vec![0.0,     2.0,
			       SQRT_2,  SQRT_2,
			       2.0,     0.0,
			       SQRT_2,  -SQRT_2,
			       0.0,     -2.0,
			       -SQRT_2, -SQRT_2,
			       -2.0,    0.0,
			       -SQRT_2, SQRT_2,
			       0.0,     2.0
	];

	let max_block_length = 8;
	let frequency = 1.0;
	let mut sample = Sample::new(sample_data, max_block_length, frequency, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, max_block_length));

	sample.note_on(frequency, 1.0);

	let mut out_left: [f32; 2] = [0.0; 2];
	let mut out_right: [f32; 2] = [0.0; 2];

	sample.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 0.0));
	assert!(f32_eq(out_right[0], 2.0));

	sample.note_on(frequency*2.0, 1.0);
	let mut out_left: [f32; 4] = [0.0; 4];
	let mut out_right: [f32; 4] = [0.0; 4];

	sample.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 2.0));
	assert!(f32_eq(out_left[1], SQRT_2 + 2.0));
	assert!(f32_eq(out_left[2], 0.0));
	assert!(f32_eq(out_left[3], -SQRT_2 - 2.0));

	assert!(sample.is_playing());

	let mut out_left: [f32; 4] = [0.0; 4];
	let mut out_right: [f32; 4] = [0.0; 4];

	sample.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], -2.0));
	assert!(f32_eq(out_left[1], -SQRT_2));
	assert!(f32_eq(out_left[2], 0.0));
	assert!(f32_eq(out_left[3], 0.0));

	assert!(!sample.is_playing());
    }

    fn make_envelope_test_sample() -> Sample {
	let sample = vec![1.0; 96];

	let max_block_length = 16;
	let frequency = 1.0;

	let mut eg = envelopes::Generator::default();
	eg.set_attack(2.0);
	eg.set_hold(3.0);
	eg.set_decay(4.0);
	eg.set_sustain(60.0);
	eg.set_release(5.0);

	Sample::new(sample, max_block_length, frequency, envelopes::ADSREnvelope::new(&eg, 1.0, max_block_length))
    }

    #[test]
    fn note_on_monophonic_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0);

	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);
    }

    #[test]
    fn sustain_monophonic_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0);
	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.6; 12]);
    }

    #[test]
    fn sustain_polyphonic_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0);

	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.6; 12]);

	sample.note_on(2.0, 1.0);

	let mut out_left = [0.0; 12];
	let mut out_right = [0.0; 12];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.6, 1.1, 1.6, 1.6, 1.6, 1.25, 1.21, 1.2, 1.2, 1.2, 1.2, 1.2]);
    }


    #[test]
    fn note_off_during_attack_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 2.0);

	let mut out_left = [0.0; 1];
	let mut out_right = [0.0; 1];

	sample.process(&mut out_left, &mut out_right);

	sample.note_off(1.0);

	let mut out_left = [0.0; 11];
	let mut out_right = [0.0; 11];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(out.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_hold_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0);

	let mut out_left = [0.0; 3];
	let mut out_right = [0.0; 3];

	sample.process(&mut out_left, &mut out_right);

	sample.note_off(1.0);

	let mut out_left = [0.0; 8];
	let mut out_right = [0.0; 8];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(out.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_decay_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0/0.65413);

	let mut out_left = [0.0; 5];
	let mut out_right = [0.0; 5];

	sample.process(&mut out_left, &mut out_right);

	sample.note_off(1.0);

	let mut out_left = [0.0; 7];
	let mut out_right = [0.0; 7];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(out.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_sustain_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0/0.6);

	let mut out_left = [0.0; 16];
	let mut out_right = [0.0; 16];

	sample.process(&mut out_left, &mut out_right);

	sample.note_off(1.0);

	let mut out_left = [0.0; 7];
	let mut out_right = [0.0; 7];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(out.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0]);
    }


    #[test]
    fn note_on_polyphonic_sample_process() {
	let mut sample = make_envelope_test_sample();

	sample.note_on(1.0, 1.0);

	let mut out_left = [0.0; 8];
	let mut out_right = [0.0; 8];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6]);

	sample.note_on(2.0, 1.0);

	let mut out_left = [0.0; 8];
	let mut out_right = [0.0; 8];

	sample.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.6, 1.1, 1.6, 1.6, 1.6, 1.25, 1.21, 1.2]);
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
