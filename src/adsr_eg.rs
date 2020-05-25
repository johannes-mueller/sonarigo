use log::error;

use crate::errors::*;


#[derive(Debug, Clone)]
pub(crate) struct Generator {
    attack: f32,
    hold: f32,
    decay: f32,
    sustain: f32,
    release: f32
}

impl Default for Generator {
    fn default() -> Self {
	Generator {
	    attack: 0.0 ,
	    hold: 0.0 ,
	    decay: 0.0 ,
	    sustain: 1.0 ,
	    release: 0.0
	}
    }
}



impl Generator {
    pub(crate) fn set_attack(&mut self, v: f32) -> Result<(), RangeError> {
	self.attack = range_check(v, 0.0, 100.0, "ampeg_attack")?;
	Ok(())
    }
    pub(crate) fn set_hold(&mut self, v: f32) -> Result<(), RangeError> {
	self.hold = range_check(v, 0.0, 100.0, "ampeg_hold")?;
	Ok(())
    }
    pub(crate) fn set_decay(&mut self, v: f32) -> Result<(), RangeError> {
	self.decay = range_check(v, 0.0, 100.0, "ampeg_decay")?;
	Ok(())
    }
    pub(crate) fn set_sustain(&mut self, v: f32) -> Result<(), RangeError> {
	self.sustain = range_check(v, 0.0, 100.0, "ampeg_sustain")? / 100.0;
	Ok(())
    }
    pub(crate) fn set_release(&mut self, v: f32) -> Result<(), RangeError> {
	self.release = range_check(v, 0.0, 100.0, "ampeg_release")?;
	Ok(())
    }

    fn ads_envelope(&self, samplerate: f32, max_block_length: usize) -> Vec<f32> {
	let needed_samples = ((self.attack + self.hold + 2.0*self.decay) as f32 * samplerate).round() as usize;
	let length = ((needed_samples / max_block_length) + 2) * max_block_length;

	let mut env = Vec::with_capacity(length);
	env.resize(length, 0.0);

	let decay_step = (-8.0/(samplerate*self.decay)).exp();
	let mut time = 0;
	let mut last = 1.0 - self.sustain;

	for e in env.iter_mut() {
	    *e = match time as f32 / samplerate {
		t if t < self.attack
		    => t/self.attack,
		t if t < self.attack + self.hold
		    => 1.0,
		t if t < self.attack + self.hold + 2.0*self.decay => {
		    last *= decay_step;
		    self.sustain + last
		}
		_ => self.sustain
	    };
	    time += 1;
	}
	env
    }

    fn sustain_envelope(&self, samplerate: f32, nsamples: usize) -> Vec<f32> {
	let mut sustain = Vec::new();
	sustain.resize(nsamples, self.sustain);
	sustain
    }

    fn release_envelope(&self, samplerate: f32, nsamples: usize) -> Vec<f32> {
	let mut env = Vec::with_capacity(nsamples);
	env.resize(nsamples, 0.0);

	let release_step = (-8.0/(samplerate*self.release)).exp();
	let mut time = 0;
	let mut last = self.sustain;

	for e in env.iter_mut() {
	    last *= release_step;
	    *e = last;
	}

	env
    }
}

#[derive(Clone, Copy)]
enum State {
    AttackDecay(usize),
    Sustain,
    Release(usize),
    Inactive
}

pub struct ADSREnvelope {
    attack_decay_envelope: Vec<f32>,
    sustain_envelope: Vec<f32>,
    release_envelope: Vec<f32>,

    max_block_length: usize,
    state: State
}

impl ADSREnvelope {
    pub(crate) fn new(generator: &Generator, samplerate: f32, max_block_length: usize) -> Self {
	ADSREnvelope {
	    attack_decay_envelope: generator.ads_envelope(samplerate, max_block_length),
	    sustain_envelope: generator.sustain_envelope(samplerate, max_block_length),
	    release_envelope: generator.release_envelope(samplerate, max_block_length),

	    max_block_length: max_block_length,
	    state: State::AttackDecay(0)
	}
    }

    pub(crate) fn active_envelope(&mut self) -> (&Vec<f32>, usize) {
	match self.state {
	    State::AttackDecay(pos) => (&self.attack_decay_envelope, pos),
	    State::Release(pos) => (&self.release_envelope, pos),
	    State::Sustain => (&self.sustain_envelope, 0),
	    State::Inactive => {
		error!("Ordered envelope while inactive. This should not happen. Using sustain.");
		(&self.sustain_envelope, 0)
	    }
	}
    }

    pub(crate) fn note_on(&mut self) {
	self.state = State::AttackDecay(0);
    }

    pub(crate) fn note_off(&mut self) {
	self.state = State::Release(0);
    }

    pub(crate) fn update(&mut self, new_pos: usize) {
	self.state = match &self.state {
	    State::AttackDecay(_) => {
		if new_pos < self.attack_decay_envelope.len() - self.max_block_length {
		    State::AttackDecay(new_pos)
		} else {
		    State::Sustain
		}
	    }
	    State::Release(_) =>  {
		if new_pos < self.release_envelope.len() - self.max_block_length {
		    State::Release(new_pos)
		} else {
		    State::Inactive
		}
	    }
	    s => *s
	}
    }

}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn generate_adsr_envelope() {
	let mut eg = Generator::default();
	eg.set_attack(2.0).unwrap();
	eg.set_hold(3.0).unwrap();
	eg.set_decay(4.0).unwrap();
	eg.set_sustain(60.0).unwrap();
	eg.set_release(5.0).unwrap();

	let ads: Vec<f32> = eg.ads_envelope(1.0, 12)[..12].iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(ads.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let rel: Vec<f32> = eg.release_envelope(1.0, 8).iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(rel.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0]);
    }
}
