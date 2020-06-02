use wmidi;

use super::envelopes;

struct Voice {
    position: f64,
    note: wmidi::Note,
    frequency: f64,
    gain: f32,

    envelope_state: envelopes::State,
    last_envelope_gain: f32,
    release_start_gain: f32
}

impl Voice {
    fn new(note: wmidi::Note, frequency: f64, gain: f32) -> Voice {
        Voice {
            frequency: frequency,
            note: note,
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

    pub fn note_on(&mut self, note: wmidi::Note, frequency: f64, gain: f32) {
        self.note_off(note);
        self.voices.push(Voice::new(note, frequency, gain))
    }

    pub fn note_off(&mut self, note: wmidi::Note) {
        for voice in &mut self.voices {
            if voice.note == note && !voice.envelope_state.is_releasing() {
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
            let needed_sample_length = (voice.position + self.max_block_length as f64 * ratio).ceil() as usize + 5;
            if  needed_sample_length * 2 >= self.sample_data.len() {
                self.sample_data.resize(needed_sample_length * 2, 0.0)
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
pub(crate) mod tests {

    use super::*;

    use std::f64::consts::PI;
    use std::f32::consts::SQRT_2;
    use std::convert::TryFrom;
    use wmidi;

    pub(crate) fn f32_eq(a: f32, b: f32) -> bool {
        if (a - b).abs() > f32::EPSILON {
            println!("float equivalence check failed, a: {}, b: {}", a, b);
            false
        } else {
            true
        }
    }

    pub fn is_playing_note(sample: &Sample, note: wmidi::Note) -> bool {
        sample.voices.iter().any(|v| v.note == note && !v.envelope_state.is_releasing())
    }

    pub fn is_releasing_note(sample: &Sample, note: wmidi::Note) -> bool {
        sample.voices.iter().any(|v| v.note == note && v.envelope_state.is_releasing())
    }

    pub(crate) fn make_test_sample_data(nsamples: usize, samplerate: f64, freq: f64) -> Vec<f32> {
        let omega = freq/samplerate * 2.0*PI;
        (0..nsamples*2).map(|t| ((omega * (t/2) as f64).sin() as f32)).collect()
    }

    pub(crate) fn make_test_sample(nsamples: usize, samplerate: f64, freq: f64) -> Sample {
        let sample_data = make_test_sample_data(nsamples, samplerate, freq);
        Sample::new(sample_data, nsamples, freq, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, nsamples))
    }


    pub(crate) fn assert_frequency(mut sample: Sample, samplerate: f64, test_freq: f64) {
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
            panic!("right frequency does not match {} {} {}", halfw_r * to_freq, (halfw_r + 1.0) * to_freq, test_freq)
        }
    }

    pub(crate) fn assert_frequency_result_sample(sample: &[f32], samplerate: f64, test_freq: f64) {
        let mut halfw = 0.0;
        let mut last = 0.0;

        for s in sample.iter() {
            if s * last < 0.0 {
                halfw += 0.5
            }
            last = *s
        }

        let to_freq = samplerate/sample.len() as f64;

        if (halfw - 1.0) * to_freq > test_freq || (halfw + 1.0) * to_freq < test_freq {
            panic!("sample frequency does not match {} {} {} / {} samples, {} {}", halfw * to_freq, (halfw + 1.0) * to_freq, test_freq, sample.len(), halfw, test_freq/to_freq)
        }
    }

    #[test]
    fn test_frequency_assertion() {
        let freq = 440.0;
        let samplerate = 48000.0;

        let omega = freq/samplerate * 2.0*PI;
        let test_sample: Vec<f32> = (0..96000).map(|t| ((omega * t as f64).sin() as f32)).collect();
        assert_frequency_result_sample(&test_sample, samplerate, freq);
    }

    #[test]
    fn sample_data_length() {
        let sample = vec![1.0, 0.5,
                          0.5, 1.0,
                          1.0, 0.5];

        let sample = Sample::new(sample, 16, 440.0, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, 16));
        assert_eq!(sample.sample_data.len(), 64);
    }

    #[test]
    fn test_test_sample_native() {
        let mut sample = make_test_sample(36000, 48000.0, wmidi::Note::A3.to_freq_f64());
        let note = wmidi::Note::A3;
        sample.note_on(note, note.to_freq_f64(), 1.0);
        assert_frequency(sample, 48000.0, 440.0);
    }

    #[test]
    fn test_test_sample_half_tone_up() {
        let mut sample = make_test_sample(36000, 48000.0, wmidi::Note::A3.to_freq_f64());
        let note = wmidi::Note::ASharp3;
        sample.note_on(note, note.to_freq_f64(), 1.0);
        assert_frequency(sample, 48000.0, 466.16);
    }

    #[test]
    fn test_test_sample_half_tone_down() {
        let mut sample = make_test_sample(36000, 48000.0, wmidi::Note::A3.to_freq_f64());
        let note = wmidi::Note::Ab3;
        sample.note_on(note, note.to_freq_f64(), 1.0);
        assert_frequency(sample, 48000.0, 415.30);
    }

    #[test]
    fn test_pitch_up_at_start() {
        let mut sample = make_test_sample(36000, 48000.0, wmidi::Note::A3.to_freq_f64());
        sample.note_on(wmidi::Note::A3, 880.0, 1.0);

        while sample.is_playing() {
            let mut out_left = [0.0; 4096];
            let mut out_right = [0.0; 4096];
            sample.process(&mut out_left, &mut out_right);
        }
    }

    #[test]
    fn test_pitch_up_late() {
        let mut sample = make_test_sample(36000, 48000.0, wmidi::Note::A3.to_freq_f64());
        sample.note_on(wmidi::Note::A3, 440.0, 1.0);

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
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();

        let mut sample = Sample::new(sample, max_block_length, frequency, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, max_block_length));

        sample.note_on(note, frequency, 1.0);

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
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();

        let mut sample = Sample::new(sample_data, max_block_length, frequency, envelopes::ADSREnvelope::new(&envelopes::Generator::default(), 1.0, max_block_length));

        sample.note_on(note, frequency, 1.0);

        let mut out_left: [f32; 2] = [0.0; 2];
        let mut out_right: [f32; 2] = [0.0; 2];

        sample.process(&mut out_left, &mut out_right);
        assert!(f32_eq(out_left[0], 0.0));
        assert!(f32_eq(out_right[0], 2.0));

        let note = wmidi::Note::C4;
        let frequency = note.to_freq_f64();
        sample.note_on(note, frequency, 1.0);

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

    fn make_envelope_test_sample(frequency: f64) -> Sample {
        let sample = vec![1.0; 96];

        let max_block_length = 16;

        let mut eg = envelopes::Generator::default();
        eg.set_attack(2.0).unwrap();
        eg.set_hold(3.0).unwrap();
        eg.set_decay(4.0).unwrap();
        eg.set_sustain(60.0).unwrap();
        eg.set_release(5.0).unwrap();

        Sample::new(sample, max_block_length, frequency, envelopes::ADSREnvelope::new(&eg, 1.0, max_block_length))
    }

    #[test]
    fn note_on_monophonic_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);

        let mut out_left = [0.0; 12];
        let mut out_right = [0.0; 12];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
        assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);
    }

    #[test]
    fn sustain_monophonic_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);
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
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);

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

        let note = wmidi::Note::C4;
        let frequency = note.to_freq_f64();
        sample.note_on(note, frequency, 1.0);

        let mut out_left = [0.0; 12];
        let mut out_right = [0.0; 12];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
        assert_eq!(out.as_slice(), [0.6, 1.1, 1.6, 1.6, 1.6, 1.25, 1.21, 1.2, 1.2, 1.2, 1.2, 1.2]);
    }


    #[test]
    fn note_off_during_attack_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 2.0);

        let mut out_left = [0.0; 1];
        let mut out_right = [0.0; 1];

        sample.process(&mut out_left, &mut out_right);

        sample.note_off(note);

        let mut out_left = [0.0; 2];
        let mut out_right = [0.0; 2];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.1211, 0.0245]);

        let mut out_left = [0.0; 9];
        let mut out_right = [0.0; 9];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_hold_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);

        let mut out_left = [0.0; 3];
        let mut out_right = [0.0; 3];

        sample.process(&mut out_left, &mut out_right);

        sample.note_off(note);

        let mut out_left = [0.0; 2];
        let mut out_right = [0.0; 2];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.1211, 0.0245]);

        let mut out_left = [0.0; 6];
        let mut out_right = [0.0; 6];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_decay_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0/0.65413);

        let mut out_left = [0.0; 5];
        let mut out_right = [0.0; 5];

        sample.process(&mut out_left, &mut out_right);

        sample.note_off(note);

        let mut out_left = [0.0; 2];
        let mut out_right = [0.0; 2];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.1211, 0.0245]);

        let mut out_left = [0.0; 5];
        let mut out_right = [0.0; 5];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.0049, 0.0010, 0.0002, 0.0, 0.0]);
    }

    #[test]
    fn note_off_during_sustain_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0/0.6);

        let mut out_left = [0.0; 16];
        let mut out_right = [0.0; 16];

        sample.process(&mut out_left, &mut out_right);

        sample.note_off(note);

        let mut out_left = [0.0; 2];
        let mut out_right = [0.0; 2];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.1211, 0.0245]);

        let mut out_left = [0.0; 5];
        let mut out_right = [0.0; 5];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
        assert_eq!(out.as_slice(), [0.0049, 0.0010, 0.0002, 0.0, 0.0]);
    }

    #[test]
    fn note_on_polyphonic_sample_process() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);

        let mut out_left = [0.0; 8];
        let mut out_right = [0.0; 8];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
        assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6]);

        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        sample.note_on(note, frequency, 1.0);

        let mut out_left = [0.0; 8];
        let mut out_right = [0.0; 8];

        sample.process(&mut out_left, &mut out_right);

        let out: Vec<f32> = out_left.iter().map(|v| (v * 10000.0).round() / 10000.0).collect();
        assert_eq!(out.as_slice(),[0.0727, 0.5147, 1.003, 1.0006, 1.0001, 0.6542, 0.6073, 0.601]);
    }

    #[test]
    fn note_on_note_on() {
        let note = wmidi::Note::C3;
        let frequency = note.to_freq_f64();
        let mut sample = make_envelope_test_sample(frequency);

        sample.note_on(note, frequency, 1.0);
        let mut out_left = [0.0; 2];
        let mut out_right = [0.0; 2];
        sample.process(&mut out_left, &mut out_right);

        assert!(sample.voices[0].envelope_state.is_active() && !sample.voices[0].envelope_state.is_releasing());
        assert_eq!(sample.voices[0].position, 2.0);
        assert!(is_playing_note(&sample, note));
        assert!(!is_releasing_note(&sample, note));

        sample.note_on(note, frequency, 1.0);
        assert!(sample.voices[0].envelope_state.is_releasing());
        assert!(sample.voices[1].envelope_state.is_active()&& !sample.voices[1].envelope_state.is_releasing());

        assert!(is_playing_note(&sample, note));
        assert!(is_releasing_note(&sample, note));

        assert_eq!(sample.voices[0].position, 2.0);
        assert_eq!(sample.voices[1].position, 0.0);

        sample.note_off(note);

        assert!(!is_playing_note(&sample, note));
        assert!(is_releasing_note(&sample, note));
    }

    #[test]
    fn note_on_off_frequencies() {
        let sample_dat = vec![1.0; 1 << 24];
        let eg = envelopes::Generator::default();
        let mut sample = Sample::new(sample_dat, 4, 1.0, envelopes::ADSREnvelope::new(&eg, 1.0, 4));

        for n in 0u8..127u8 {
            let note = wmidi::Note::try_from(n).unwrap();
            sample.note_on(note, note.to_freq_f64(), 1.0);
            assert!(is_playing_note(&sample, note));
        }
        for n in 0u8..127u8 {
            let note = wmidi::Note::try_from(n).unwrap();
            sample.note_off(note);
            let mut out_left = [0.0; 2];
            let mut out_right = [0.0; 2];
            sample.process(&mut out_left, &mut out_right);
            assert!(!is_playing_note(&sample, note));
        }
        assert!(!sample.is_playing());
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
