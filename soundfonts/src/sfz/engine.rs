use std::collections::{HashSet, HashMap};
use std::convert::TryFrom;
use std::path::Path;
use std::fmt;
use std::error;
use std::io;

use crate::errors::*;
use crate::engine;
use crate::sample;
use crate::envelopes;
use crate::utils;
use crate::sndfile;
use crate::sndfile::SndFileIO;

use super::parser;

#[derive(Clone, Copy)]
pub(super) struct VelRange {
    lo: wmidi::Velocity,
    hi: wmidi::Velocity
}

impl VelRange {
    pub(super) fn set_hi(&mut self, v: i32) -> Result<(), RangeError> {
	let vel = wmidi::Velocity::try_from(v as u8).map_err(|_| RangeError::out_of_range("hivel", 0, 127, v))?;
	if  vel < self.lo {
	    return Err(RangeError::flipped_range("hivel", v, u8::from(self.lo) as i32));
	}
	self.hi = vel;
	Ok(())
    }

    pub(super) fn set_lo(&mut self, v: i32) -> Result<(), RangeError> {
	let vel = wmidi::Velocity::try_from(v as u8).map_err(|_| RangeError::out_of_range("lovel", 0, 127, v))?;
	if  vel > self.hi {
	    return Err(RangeError::flipped_range("lovel", v, u8::from(self.hi) as i32));
	}
	self.lo = vel;
	Ok(())
    }

    pub(super) fn covering(&self, vel: wmidi::Velocity) -> bool {
	vel >= self.lo && vel <= self.hi
    }
}


impl Default for VelRange {
    fn default() -> Self {
	VelRange {
	    hi: wmidi::Velocity::MAX,
	    lo: wmidi::Velocity::MIN
	}
    }
}

#[derive(Clone, Copy)]
pub(super) struct NoteRange {
    lo: Option<wmidi::Note>,
    hi: Option<wmidi::Note>
}

impl NoteRange {
    pub(super) fn set_hi(&mut self, v: i32) -> Result<(), RangeError> {
	if v == -1 {
	    self.hi = None;
	    return Ok(());
	}

	let note = wmidi::Note::try_from(v as u8).map_err(|_| RangeError::out_of_range("hikey", -1, 127, v))?;
	if self.lo.map_or(false, |n| note < n) {
	    return Err(RangeError::flipped_range("hikey", v, u8::from(note) as i32));
	}
	self.hi = Some(note);
	Ok(())
    }

    pub(super) fn set_lo(&mut self, v: i32) -> Result<(), RangeError> {
	if v == -1 {
	    self.lo = None;
	    return Ok(());
	}

	let note = wmidi::Note::try_from(v as u8).map_err(|_| RangeError::out_of_range("lokey", -1, 127, v))?;
	if self.hi.map_or(false, |n| note > n) {
	    return Err(RangeError::flipped_range("lokey", v, u8::from(note) as i32));
	}
	self.lo = Some(note);
	Ok(())
    }

    pub(super) fn covering(&self, note: wmidi::Note) -> bool {
	match (self.lo, self.hi) {
	    (Some(lo), Some(hi)) => note >= lo && note <= hi,
	     _ => false
	}
    }
}


impl Default for NoteRange {
    fn default() -> Self {
	NoteRange {
	    hi: Some(wmidi::Note::HIGHEST_NOTE),
	    lo: Some(wmidi::Note::LOWEST_NOTE)
	}
    }
}


#[derive(Default, Clone)]
pub(super) struct RandomRange {
    hi: f32,
    lo: f32
}

impl RandomRange {
    pub(super) fn set_hi(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v < 0.0 && v > 1.0 => Err(RangeError::out_of_range("hirand", "0.0", "1.0", v.to_string().as_str())),
	    v if v < self.lo && self.lo > 0.0 => Err(RangeError::flipped_range("hirand", v.to_string().as_str(), self.lo.to_string().as_str())),
	    _ => {
		self.hi = v;
		Ok(())
	    }
	}
    }

    pub(super) fn set_lo(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v < 0.0 && v > 1.0 => Err(RangeError::out_of_range("lorand", 0.0, 1.0, v)),
	    v if v > self.hi && self.hi > 0.0 => Err(RangeError::flipped_range("lorand", v, self.hi)),
	    _ => {
		self.lo = v;
		Ok(())
	    }
	}
    }

    fn covering(&self, v: f32) -> bool {
	self.hi == self.lo || (v >= self.lo && v < self.hi)
    }
}

#[derive(Default, Clone)]
pub(super) struct ControlValRange {
    hi: Option<wmidi::ControlValue>,
    lo: Option<wmidi::ControlValue>
}

impl ControlValRange {
    pub(super) fn set_hi(&mut self, v: i32) -> Result<(), RangeError> {
	if v < 0 {
	    self.hi = None;
	    return Ok(());
	}
	let val = wmidi::ControlValue::try_from(v as u8).map_err(|_| RangeError::out_of_range("on_hiccXX", 0, 127, v))?;
	match self.lo {
	    Some(lo) if val < lo => {
		return Err(RangeError::flipped_range("on_hiccXX", v, u8::from(lo) as i32));
	    }
	    _ => {}
	};
	self.hi = Some(val);
	Ok(())
    }

    pub(super) fn set_lo(&mut self, v: i32) -> Result<(), RangeError> {
	if v < 0 {
	    self.lo = None;
	    return Ok(());
	}
	let val = wmidi::ControlValue::try_from(v as u8).map_err(|_| RangeError::out_of_range("on_loccXX", 0, 127, v))?;
	match self.hi {
	    Some(hi) if val > hi => {
		return Err(RangeError::flipped_range("on_loccXX", v, u8::from(hi) as i32));
	    }
	    _ => {}
	};
	self.lo = Some(val);
	Ok(())
    }

    pub(super) fn covering(&self, vel: wmidi::ControlValue) -> bool {
	match (self.lo, self.hi) {
	    (Some(lo), Some(hi)) => vel >= lo && vel <= hi,
	     _ => false
	}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Trigger {
    Attack,
    Release,
    First,
    Legato,
    ReleaseKey
}

impl Default for Trigger {
    fn default() -> Self {
	Trigger::Attack
    }
}



#[derive(Clone)]
pub struct RegionData {
    pub(super) key_range: NoteRange,
    pub(super) vel_range: VelRange,

    pub(super) ampeg: envelopes::Generator,

    pitch_keycenter: wmidi::Note,

    pitch_keytrack: f64,

    amp_veltrack: f32,

    volume: f32,

    sample: String,
    rt_decay: f32,

    tune: f64,

    trigger: Trigger,

    group: u32,
    off_by: u32,

    on_ccs: HashMap<u8, ControlValRange>,

    pub(super) random_range: RandomRange,
}


impl Default for RegionData {
    fn default() -> Self {
	RegionData {
	    key_range: Default::default(),
	    vel_range: Default::default(),

	    pitch_keycenter: wmidi::Note::C3,

	    pitch_keytrack: 1.0,

	    amp_veltrack: 1.0,

	    ampeg: Default::default(),

	    volume: Default::default(),
	    sample: Default::default(),
	    rt_decay: Default::default(),
	    tune: Default::default(),
	    trigger: Default::default(),

	    group:  Default::default(),
	    off_by:  Default::default(),

	    on_ccs: HashMap::new(),

	    random_range: Default::default(),
	}
    }
}

impl RegionData {
    pub(super) fn set_amp_veltrack(&mut self, v: f32) -> Result<(), RangeError> {
	self.amp_veltrack = range_check(v, -100.0, 100.0, "amp_veltrack")? / 100.0;
	Ok(())
    }

    pub(super) fn set_pitch_keycenter(&mut self, v: u32) -> Result<(), RangeError> {
	let v = range_check(v, 0, 127, "pich_keycenter")? as u8;
	self.pitch_keycenter = unsafe { wmidi::Note::from_u8_unchecked(v as u8) };
	Ok(())
    }

    pub(super) fn set_pitch_keytrack(&mut self, v: f32) -> Result<(), RangeError> {
	self.pitch_keytrack = range_check(v as f64, -1200.0, 1200.0, "pitch_keytrack")? / 100.0;
	Ok(())
    }

    pub(super) fn set_sample(&mut self, v: &str) {
	self.sample = v.to_string();
    }

    pub(super) fn set_rt_decay(&mut self, v: f32) -> Result<(), RangeError> {
	self.rt_decay = range_check(v, 0.0, 200.0, "rt_decay")?;
	Ok(())
    }

    pub(super) fn set_tune(&mut self, v: i32) -> Result<(), RangeError> {
	self.tune = range_check(v, -100, 100, "tune")? as f64 / 100.0;
	Ok(())
    }

    pub(super) fn set_volume(&mut self, v: f32) -> Result<(), RangeError> {
	self.volume = range_check(v, -144.6, 6.0, "tune")?;
	Ok(())
    }

    pub(super) fn set_trigger(&mut self, t: Trigger) {
	self.trigger = t;
    }

    pub(super) fn set_group(&mut self, v: u32) {
	self.group = v;
    }

    pub(super) fn set_off_by(&mut self, v: u32) {
	self.off_by = v;
    }

    pub(super) fn push_on_lo_cc(&mut self, channel: u32, v: i32) -> Result<(), RangeError> {
	let channel = channel as u8;
	match self.on_ccs.get_mut(&channel) {
	    Some(ref mut range) => range.set_lo(v),
	    None => {
		let mut range = ControlValRange { hi: None, lo: None };
		range.set_lo(v)?;
		self.on_ccs.insert(channel, range);
		Ok(())
	    }
	}
    }

    pub(super) fn push_on_hi_cc(&mut self, channel: u32, v: i32) -> Result<(), RangeError> {
	let channel = channel as u8;
	match self.on_ccs.get_mut(&channel) {
	    Some(ref mut range) => range.set_hi(v),
	    None => {
		let mut range = ControlValRange { hi: None, lo: None };
		range.set_hi(v)?;
		self.on_ccs.insert(channel, range);
		Ok(())
	    }
	}
    }

}


pub(super) struct Region {
    params: RegionData,

    sample: sample::Sample,

    gain: f32,

    samplerate: f64,

    last_note_on: Option<(wmidi::Note, wmidi::Velocity)>,
    notes_for_release_trigger: HashSet<wmidi::Note>,

    other_notes_on: HashSet<u8>,
    time_since_note_on: f64,

    sustain_pedal_pushed: bool,

    once_immune_against_group_events: bool,
}

impl Region {
    fn new(params: RegionData, sample_data: Vec<f32>, samplerate: f64, max_block_length: usize) -> Region {

	let amp_envelope = envelopes::ADSREnvelope::new(&params.ampeg, samplerate as f32, max_block_length);
	let sample = sample::Sample::new(sample_data, max_block_length, params.pitch_keycenter.to_freq_f64(), amp_envelope);

	Region {
	    params: params,

	    sample: sample,

	    gain: 1.0,

	    samplerate: samplerate,

	    last_note_on: None,
	    notes_for_release_trigger: HashSet::new(),
	    other_notes_on: HashSet::new(),
	    time_since_note_on: 0.0,

	    sustain_pedal_pushed: false,

	    once_immune_against_group_events: false
	}
    }

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
	self.time_since_note_on += out_left.len() as f64 / self.samplerate;

	if !self.sample.is_playing() {
	    return;
	}
	self.sample.process(out_left, out_right);

	/*
	for (l, r, (sl, sr)) in izip!(out_left.iter_mut(), out_right.iter_mut(), sample_iterator) {
	    *l += sl * self.gain * envelope[env_position];
	    *r += sr * self.gain * envelope[env_position];

	    env_position += 1;
	}


	self.amp_envelope.update(env_position);
*/

    }

    fn is_playing_note(&self, note: wmidi::Note) -> bool {
	self.sample.is_playing_note(note)
    }

    fn note_on(&mut self, note: wmidi::Note, velocity: wmidi::Velocity) {
	println!("note on {}", note);
	let velocity = u8::from(velocity);
	let vel = if self.params.amp_veltrack < 0.0 {
	    127 - velocity
	} else {
	    velocity
	};

	let velocity_db = if vel == 0 {
	    -160.0
	} else {
	    let vel = vel as f32;
	    -20.0 * ((127.0 * 127.0)/(vel * vel)).log10()
	};

	let rt_decay = match self.params.trigger {
	    Trigger::Release |
	    Trigger::ReleaseKey => self.time_since_note_on as f32 * (-self.params.rt_decay),
	    _ => 0.0
	};

	self.gain = utils::dB_to_gain(self.params.volume + velocity_db * self.params.amp_veltrack.abs() + rt_decay);

	let native_freq = self.params.pitch_keycenter.to_freq_f64();

	let current_note_frequency = native_freq * (note.to_freq_f64()/native_freq).powf(self.params.pitch_keytrack) * 2.0f64.powf(1.0/12.0 * self.params.tune);

	self.time_since_note_on = 0.0;
	self.sample.note_on(note, current_note_frequency, self.gain);
    }

    fn note_off(&mut self, note: wmidi::Note) {
	self.sample.note_off(note);
    }

    fn sustain_pedal(&mut self, pushed: bool) {
	self.sustain_pedal_pushed = pushed;

	if !pushed {
	    match self.params.trigger {
		Trigger::Release => self.last_note_on.map_or((), |(note, velocity)| self.note_on(note, velocity)),
		_ => {
		    for note in self.notes_for_release_trigger.clone() {
			self.note_off(note);
		    }
		    self.notes_for_release_trigger.clear();
		}
	    }
	}
    }

    fn handle_note_on(&mut self, note: wmidi::Note, velocity: wmidi::Velocity) -> bool {
	if self.is_playing_note(note) && !self.sample.is_releasing_note(note) {
	    return false;
	}

	if !self.params.key_range.covering(note) {
	    self.other_notes_on.insert(u8::from(note));
	    return false;
	}

	if !self.params.vel_range.covering(velocity) {
	    return false;
	}

 	match self.params.trigger {
	    Trigger::Release |
	    Trigger::ReleaseKey => {
		self.last_note_on = Some((note, velocity));
		return false
	    }
	    Trigger::First => {
		if !self.other_notes_on.is_empty() {
		    return false;
		}
	    }
	    Trigger::Legato => {
		if self.other_notes_on.is_empty() {
		    return false;
		}
	    }
	    _ => {}
	}
	self.note_on(note, velocity);
	true
    }

    fn handle_note_off(&mut self, note: wmidi::Note) -> bool {
	if !self.params.key_range.covering(note) {
	    self.other_notes_on.remove(&u8::from(note));
	    return false;
	}
	match self.params.trigger {
	    Trigger::Release |
	    Trigger::ReleaseKey => {
		match self.last_note_on {
		    Some((note, velocity)) => {
			self.note_on(note, velocity);
			true
		    }
		    None => false
		}
	    }
	    _ => {
		if !self.sustain_pedal_pushed {
		    self.note_off(note);
		} else {
		    self.notes_for_release_trigger.insert(note);
		}
		false
	    }
	}
    }

    fn handle_control_event(&mut self, control_number: wmidi::ControlNumber, control_value: wmidi::ControlValue) -> bool {
	let (cnum, cval) = (u8::from(control_number), u8::from(control_value));

	match cnum {
	    64 => self.sustain_pedal(cval >= 64),
	    _ => {}
	}

	match self.params.on_ccs.get(&cnum) {
	    Some(cvrange) if cvrange.covering(control_value) => {
		self.note_on(self.params.pitch_keycenter, wmidi::Velocity::MAX);
		true
	    }
	    _ => false
	}
    }

    fn pass_midi_msg(&mut self, midi_msg: &wmidi::MidiMessage, random_value: f32) -> bool {
	self.once_immune_against_group_events = false;
	match midi_msg {
	    wmidi::MidiMessage::NoteOn(_ch, note, vel) => {
		if self.params.random_range.covering(random_value) {
		    self.handle_note_on(*note, *vel)
		} else {
		    false
		}
	    }
	    wmidi::MidiMessage::NoteOff(_ch, note, _vel) => self.handle_note_off(*note),
	    wmidi::MidiMessage::ControlChange(_ch, cnum, cval) => self.handle_control_event(*cnum, *cval),
	    _ => false
	}
    }

    fn group(&mut self) -> u32 {
	self.once_immune_against_group_events = true;
	self.params.group
    }

    fn group_activated(&mut self, group: u32) {
	if self.once_immune_against_group_events {
	    return;
	}
	if group == self.params.group || group == self.params.off_by {
	    self.sample.all_notes_off();
	}
    }
}

#[derive(Debug)]
pub enum EngineError {
    ParserError(parser::ParserError),
    SndFileError(sndfile::SndFileError),
    IOError(io::Error),
    UnspecifiedSndFileError(String)

}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	match &*self {
	    EngineError::ParserError(pe) => std::fmt::Display::fmt(&pe, f),
	    EngineError::SndFileError(sfe) => fmt::Debug::fmt(&sfe, f),
	    EngineError::IOError(ioe) => fmt::Display::fmt(&ioe, f),
	    EngineError::UnspecifiedSndFileError(sf) => write!(f, "Unspecified error from sndfile while reading {}", sf)
	}
    }
}

impl error::Error for EngineError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
	match *self {
	    EngineError::ParserError(ref e) => Some(e),
	    EngineError::SndFileError(_) => None, // SndFileError should implement std::errer::Error
	    EngineError::IOError(ref e) => Some(e),
	    _ => None
	}
    }
}


pub struct Engine {
    pub(super) regions: Vec<Region>,
}

impl Engine {
    pub fn new(sfz_file: String, samplerate: f64, max_block_length: usize) -> Result<Engine, EngineError> {
	let mut fh = std::fs::File::open(&sfz_file).map_err(|e| EngineError::IOError(e))?;
	let mut sfz_text = String::new();
	io::Read::read_to_string(&mut fh, &mut sfz_text).map_err(|e| EngineError::IOError(e))?;

	let region_data = parser::parse_sfz_text(sfz_text).map_err(|pe| EngineError::ParserError(pe))?;

	let sample_path = Path::new(&sfz_file).parent().unwrap();

	let regions: Result<Vec<(RegionData, Vec<f32>)>, _> = region_data.iter().map( |rd| {
	    let sample_file = rd.sample.replace("\\", &std::path::MAIN_SEPARATOR.to_string());
	    println!("{}", sample_file);
	    let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto).from_path(sample_path.join(&sample_file))
		.map_err(|sfe| EngineError::SndFileError(sfe))?;
	    let sample = snd.read_all_to_vec().map_err(|_| EngineError::UnspecifiedSndFileError(sample_file))?;
	    Ok((rd.clone(), sample))
	}).collect();
	println!("loaded");
	regions.map(|data| Self::from_region_array(data, samplerate, max_block_length))
    }

    fn from_region_array(reg_data_sample: Vec<(RegionData, Vec<f32>)>, samplerate: f64, max_block_length: usize) -> Engine {
	Engine {
	    regions: reg_data_sample.iter().map(|(rd, sample)| Region::new(rd.clone(), sample.to_vec(), samplerate, max_block_length)).collect(),
	}
    }
}

impl engine::EngineTrait for Engine {
    fn midi_event(&mut self, midi_msg: &wmidi::MidiMessage) {
	let mut activated_groups = HashSet::new();
	let random_value = rand::random();
	for r in &mut self.regions {
	    if r.pass_midi_msg(midi_msg, random_value) {
		let group = r.group();
		if group > 0 {
		    activated_groups.insert(group);
		}
	    }
	}
	for group in activated_groups {
	    for r in &mut self.regions {
		r.group_activated(group);
	    }
	}
    }

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
	for (l, r) in Iterator::zip(out_left.iter_mut(), out_right.iter_mut()) {
	    *l = 0.0;
	    *r = 0.0;
	}
	for r in &mut self.regions {
	    r.process(out_left, out_right);
	}
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::super::parser::parse_sfz_text;
    use crate::engine::EngineTrait;

    use crate::sndfile;
    use crate::sndfile::SndFileIO;

    use crate::sample::tests as sampletests;
    use crate::sample::tests::f32_eq;

    #[test]
    fn region_data_default() {
	let rd: RegionData = Default::default();

	assert_eq!(rd.key_range.hi, Some(wmidi::Note::HIGHEST_NOTE));
	assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
	assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);

	assert_eq!(rd.amp_veltrack, 1.0);
/* FIXME: How to test this?
	let mut env = envelopes::ADSREnvelope::new(&rd.ampeg, 1.0, 4);
	let (sustain_env, _) = env.active_envelope();
	assert_eq!(*sustain_env.as_slice(), [1.0; 4]);
*/
	assert_eq!(rd.tune, 0.0)
    }

    #[test]
    fn parse_empty_text() {
	match parse_sfz_text("".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "General parser error: Expecting <> tag in sfz file"),
	    _ => panic!("Expected error message")
	}
    }

    #[test]
    fn parse_sfz_hikey_lokey_region_line() {
	let regions = parse_sfz_text("<region> hikey=42 lokey=23".to_string()).unwrap();
	assert_eq!(regions.len(), 1);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_sfz_key_region_line() {
	let regions = parse_sfz_text("<region> key=42".to_string()).unwrap();
	assert_eq!(regions.len(), 1);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::FSharp1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_sfz_hikey_lokey_notefmt_region_line() {
	let regions = parse_sfz_text("<region> hikey=c#3 lokey=ab2 <region> hikey=c3 lokey=a2".to_string()).unwrap();
	assert_eq!(regions.len(), 2);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::Db2));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::GSharp1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::C2));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::A1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_sfz_hikey_lokey_group_line() {
	let regions = parse_sfz_text("<group> hivel=42 lovel=23".to_string()).unwrap();
	assert_eq!(regions.len(), 0);
    }

    #[test]
    fn parse_sfz_invalid_header_line() {
	match parse_sfz_text("<foo> hikey=42 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Unknown key: foo"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_invalid_opcode_line() {
	match parse_sfz_text("<region> foo=42 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Unknown key: foo"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_invalid_non_int_value_line() {
	match parse_sfz_text("<region> hikey=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Invalid key: aa"),
	    _ => panic!("Not seen expected error")
	}
    }

    /* FIXME: How to test this?
    #[test]
    fn parse_ampeg() {
	let regions = parse_sfz_text("<region> ampeg_attack=23 ampeg_hold=42 ampeg_decay=47 ampeg_sustain=11 ampeg_release=0.2342".to_string()).unwrap();
	match regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.ampeg.attack, 23.0);
		assert_eq!(rd.ampeg.hold, 42.0);
		assert_eq!(rd.ampeg.decay, 47.0);
		assert_eq!(rd.ampeg.sustain, 0.11);
		assert_eq!(rd.ampeg.release, 0.2342);
	    }
	    None => panic!("expeted region with ampeg")
	}
    }
     */

    #[test]
    fn parse_out_of_range_amp_veltrack() {
	match parse_sfz_text("<region> amp_veltrack=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "amp_veltrack out of range: -100 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> amp_veltrack=-105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "amp_veltrack out of range: -100 <= -105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_ampeg_attack() {
	match parse_sfz_text("<region> ampeg_attack=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_attack out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_attack=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_attack out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_attack=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

        #[test]
    fn parse_out_of_range_ampeg_hold() {
	match parse_sfz_text("<region> ampeg_hold=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_hold out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_hold=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_hold out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_hold=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_ampeg_decay() {
	match parse_sfz_text("<region> ampeg_decay=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_decay out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_decay=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_decay out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_decay=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_ampeg_sustain() {
	match parse_sfz_text("<region> ampeg_sustain=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_sustain out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_sustain=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_sustain out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_sustain=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_ampeg_release() {
	match parse_sfz_text("<region> ampeg_release=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_release out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_release=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_release out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parse_sfz_text("<region> ampeg_release=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_comment_in_line() {
	let regions = parse_sfz_text("<region> hivel=42 lovel=23 // foo".to_string()).unwrap();
	assert_eq!(regions.len(), 1);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::HIGHEST_NOTE));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 23);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_region_line_span() {
	let regions = parse_sfz_text("<region> hivel=42 lovel=23 \n hikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(regions.len(), 1);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::C0));
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 23);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_region_line_span_with_coment() {
	let regions = parse_sfz_text("<region> hivel=42 lovel=23 // foo bar foo\nhikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(regions.len(), 1);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::C0));
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 23);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_two_region_line() {
	let s = "<region> hivel=41 lovel=22 <region> hikey=42 lokey=23";

	let regions = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(regions.len(), 2)
    }

    #[test]
    fn parse_regions_inheriting_group_data() {
	let s = "
<group> hivel=42
<region> lovel=23
<region> lovel=21
";
	let regions = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(regions.len(), 2);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 23)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 21)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_regions_inheriting_group_data_2groups() {
	let s = "
<group> hivel=42 hikey=41
<region> lokey=23
<region> lovel=21
<group> hikey=42 hivel=41
<region> lokey=23
<region> lovel=21
<region> hikey=43 hivel=42 lokey=23
<region> lovel=23
";
	let regions = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(regions.len(), 6);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(u8::from(rd.vel_range.lo), 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(2) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 41);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(3) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 41);
		assert_eq!(u8::from(rd.vel_range.lo), 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(4) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 42);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(5) {
	    Some(rd) => {
		assert_eq!(u8::from(rd.vel_range.hi), 41);
		assert_eq!(u8::from(rd.vel_range.lo), 23);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_shortened_real_life_sfz() {
	let s = r#"
//=====================================
// Salamander Grand Piano V2
// (only a small part for testing the parser)
// Author: Alexander Holm
// Contact: axeldenstore [at] gmail [dot] com
// License: CC-by
//
//=====================================

//Notes
<group> amp_veltrack=73 ampeg_release=1

<region> sample=48khz24bit\A0v1.wav lokey=21 hikey=22 lovel=1 hivel=26 pitch_keycenter=21 tune=10
<region> sample=48khz24bit\A0v2.wav lokey=21 hikey=22 lovel=27 hivel=34 pitch_keycenter=21 tune=10

//========================
//Notes without dampers
<group> amp_veltrack=73 ampeg_release=5

<region> sample=48khz24bit\F#6v1.wav lokey=89 hikey=91 lovel=1 hivel=26 pitch_keycenter=90 tune=-13
<region> sample=48khz24bit\F#6v2.wav lokey=89 hikey=91 lovel=27 hivel=34 pitch_keycenter=90 tune=-13
//Release string resonances
<group> trigger=release volume=-4 amp_veltrack=94 rt_decay=6

<region> sample=48khz24bit\harmLA0.wav lokey=20 hikey=22 lovel=45 pitch_keycenter=21
<region> sample=48khz24bit\harmLC1.wav lokey=23 hikey=25 lovel=45 pitch_keycenter=24

//======================
//HammerNoise
<group> trigger=release pitch_keytrack=0 volume=-37 amp_veltrack=82 rt_decay=2

<region> sample=48khz24bit\rel1.wav lokey=21 hikey=21
<region> sample=48khz24bit\rel2.wav lokey=22 hikey=22
//======================
//pedalAction

<group> group=1 hikey=-1 lokey=-1 on_locc64=126 on_hicc64=127 off_by=2 volume=-20

<region> sample=48khz24bit\pedalD1.wav lorand=0 hirand=0.5
<region> sample=48khz24bit\pedalD2.wav lorand=0.5 hirand=1

<group> group=2 hikey=-1 lokey=-1 on_locc64=0 on_hicc64=1 volume=-19

<region> sample=48khz24bit\pedalU1.wav lorand=0 hirand=0.5
<region> sample=48khz24bit\pedalU2.wav lorand=0.5 hirand=1

"#;
	let regions = parse_sfz_text(s.to_string()).unwrap();

	assert_eq!(regions.len(), 12);
	match &regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 1.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::AMinus1);
		assert_eq!(rd.tune, 0.1);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(u8::from(rd.vel_range.hi), 26);
		assert_eq!(u8::from(rd.vel_range.lo), 1);
		assert_eq!(rd.sample, "48khz24bit\\A0v1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, 0.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 1.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::AMinus1);
		assert_eq!(rd.tune, 0.1);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(u8::from(rd.vel_range.hi), 34);
		assert_eq!(u8::from(rd.vel_range.lo), 27);
		assert_eq!(rd.sample, "48khz24bit\\A0v2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, 0.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(2) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 5.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::Gb5);
		assert_eq!(rd.tune, -0.13);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G5));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::F5));
		assert_eq!(u8::from(rd.vel_range.hi), 26);
		assert_eq!(u8::from(rd.vel_range.lo), 1);
		assert_eq!(rd.sample, "48khz24bit\\F#6v1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, 0.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(3) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 5.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::Gb5);
		assert_eq!(rd.tune, -0.13);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G5));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::F5));
		assert_eq!(u8::from(rd.vel_range.hi), 34);
		assert_eq!(u8::from(rd.vel_range.lo), 27);
		assert_eq!(rd.sample, "48khz24bit\\F#6v2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, 0.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(4) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.94);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::AMinus1);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AbMinus1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(u8::from(rd.vel_range.lo), 45);
		assert_eq!(rd.sample, "48khz24bit\\harmLA0.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 6.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -4.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(5) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.94);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C0);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::Db0));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(u8::from(rd.vel_range.lo), 45);
		assert_eq!(rd.sample, "48khz24bit\\harmLC1.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 6.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -4.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(6) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.82);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\rel1.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 2.0);
		assert_eq!(rd.pitch_keytrack, 0.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -37.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(7) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.82);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::ASharpMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::ASharpMinus1));
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\rel2.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 2.0);
		assert_eq!(rd.pitch_keytrack, 0.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.is_empty(), (0, 0));
		assert_eq!(rd.random_range.hi, 0.0);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -37.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(8) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\pedalD1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 1);
		assert_eq!(rd.off_by, 2);
		assert!(rd.on_ccs.get(&64).unwrap().covering(wmidi::ControlValue::try_from(126).unwrap()));
		assert_eq!(rd.random_range.hi, 0.5);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -20.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(9) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\pedalD2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 1);
		assert_eq!(rd.off_by, 2);
		assert!(rd.on_ccs.get(&64).unwrap().covering(wmidi::ControlValue::try_from(127).unwrap()));
		assert_eq!(rd.random_range.hi, 1.0);
		assert_eq!(rd.random_range.lo, 0.5);
		assert_eq!(rd.volume, -20.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(10) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\pedalU1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 2);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.get(&64).unwrap().covering(wmidi::ControlValue::try_from(1).unwrap()));
		assert_eq!(rd.random_range.hi, 0.5);
		assert_eq!(rd.random_range.lo, 0.0);
		assert_eq!(rd.volume, -19.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(11) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		// FIXME: how to test this? assert_eq!(rd.ampeg.release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C3);
		assert_eq!(rd.tune, 0.0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, wmidi::Velocity::MAX);
		assert_eq!(rd.vel_range.lo, wmidi::Velocity::MIN);
		assert_eq!(rd.sample, "48khz24bit\\pedalU2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 1.0);
		assert_eq!(rd.group, 2);
		assert_eq!(rd.off_by, 0);
		assert!(rd.on_ccs.get(&64).unwrap().covering(wmidi::ControlValue::try_from(0).unwrap()));
		assert_eq!(rd.random_range.hi, 1.0);
		assert_eq!(rd.random_range.lo, 0.5);
		assert_eq!(rd.volume, -19.0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    /*
    #[test]
    fn generate_adsr_envelope() {
	let regions = parse_sfz_text("<region> ampeg_attack=2 ampeg_hold=3 ampeg_decay=4 ampeg_sustain=60 ampeg_release=5".to_string()).unwrap();
	let region = regions.get(0).unwrap();

	let ads: Vec<f32> = region.ampeg.ads_envelope(1.0, 12)[..12].iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(ads.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let rel: Vec<f32> = region.ampeg.release_envelope(1.0, 8).iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(rel.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0]);
    }
    */

    #[test]
    fn simple_region_process() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let mut region = Region::new(RegionData::default(), sample, 1.0, 8);

	region.note_on(wmidi::Note::C3, wmidi::Velocity::MAX);

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	region.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 1.0));
	assert!(f32_eq(out_left[1], 0.5));

	assert!(f32_eq(out_right[0], 0.5));
	assert!(f32_eq(out_right[1], 1.0));

	assert!(region.is_playing_note(wmidi::Note::C3));

	let mut out_left: [f32; 2] = [-0.5, -0.2];
	let mut out_right: [f32; 2] = [-0.2, -0.5];

	region.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 0.5));
	assert!(f32_eq(out_left[1], -0.2));

	assert!(f32_eq(out_right[0], 0.3));
	assert!(f32_eq(out_right[1], -0.5));

	assert!(!region.is_playing_note(wmidi::Note::C3));
    }

    #[test]
    fn region_volume_process() {
	let sample = vec![1.0, 1.0];

	let mut region_data = RegionData::default();
	region_data.set_volume(-20.0).unwrap();

	let mut region = Region::new(region_data, sample, 1.0, 8);

	region.note_on(wmidi::Note::C3, wmidi::Velocity::MAX);

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	region.process(&mut out_left, &mut out_right);

	assert_eq!(out_left[0], 0.1);
	assert_eq!(out_right[0], 0.1);
    }

    #[test]
    fn region_amp_envelope_process() {
	let mut sample = vec![];
	sample.resize(32, 1.0);
	let regions = parse_sfz_text("<region> ampeg_attack=2 ampeg_hold=3 ampeg_decay=4 ampeg_sustain=60 ampeg_release=5".to_string()).unwrap();

	let mut region = Region::new(regions.get(0).unwrap().clone(), sample, 1.0, 16);
	region.note_on(wmidi::Note::C3, wmidi::Velocity::MAX);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);
    }

  #[test]
    fn region_amp_envelope_process_sustain() {
	let sample = vec![1.0; 96];

	let regions = parse_sfz_text("<region> ampeg_attack=2 ampeg_hold=3 ampeg_decay=4 ampeg_sustain=60 ampeg_release=5".to_string()).unwrap();

	let mut region = Region::new(regions.get(0).unwrap().clone(), sample, 1.0, 12);
	region.note_on(wmidi::Note::C3, wmidi::Velocity::MAX);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);
	let out: Vec<f32> = out_left.iter().map(|v| (v*1000.0).round()/1000.0).collect();
	assert_eq!(out, [0.6; 12]);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);
	let out: Vec<f32> = out_left.iter().map(|v| (v*1000.0).round()/1000.0).collect();
	assert_eq!(out, [0.6; 12]);

    	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);
	let out: Vec<f32> = out_left.iter().map(|v| (v*1000.0).round()/1000.0).collect();
	assert_eq!(out, [0.6; 12]);
    }


    #[test]
    fn engine_process_silence() {
	let mut engine = Engine::from_region_array(vec![(RegionData::default(), Vec::new()), (RegionData::default(), Vec::new())], 1.0, 16);

	let mut out_left: [f32; 4] = [1.0; 4];
	let mut out_right: [f32; 4] = [1.0; 4];

	engine.process(&mut out_left, &mut out_right);

	assert_eq!(out_left, [0.0; 4]);
	assert_eq!(out_right, [0.0; 4]);
    }


    #[test]
    fn simple_engine_process() {
	let sample1 = vec![1.0, 0.5,
			   0.5, 1.0,
			   1.0, 0.5];
	let sample2 = vec![-0.5, 0.5,
			   -0.5, -0.5,
			   0.0, 0.5];

	let mut engine = Engine::from_region_array(vec![(RegionData::default(), sample1), (RegionData::default(), sample2)], 1.0, 16);

	engine.regions[0].note_on(wmidi::Note::C3, wmidi::Velocity::MAX);
	engine.regions[1].note_on(wmidi::Note::C3, wmidi::Velocity::MAX);

	let mut out_left: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
	let mut out_right: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

	engine.process(&mut out_left, &mut out_right);

	assert!(!engine.regions[0].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[1].is_playing_note(wmidi::Note::C3));

	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_left[1], 0.0);
	assert_eq!(out_left[2], 1.0);

	assert_eq!(out_right[0], 1.0);
	assert_eq!(out_right[1], 0.5);
	assert_eq!(out_right[2], 1.0);
    }

    fn make_dummy_region(rd: RegionData, samplerate: f64, max_block_length: usize) -> Region {
	let sample = vec![1.0; 96];
	Region::new(rd, sample, samplerate, max_block_length)
    }

    fn pull_samples(region: &mut Region, nsamples: usize) -> (Vec<f32>, Vec<f32>) {
	let mut out_left = Vec::new();
	out_left.resize(nsamples, 0.0);
	let mut out_right = Vec::new();
	out_right.resize(nsamples, 0.0);

	region.process(&mut out_left, &mut out_right);
	(out_left, out_right)
    }

    #[test]
    fn note_trigger_key_range() {
	let mut rd = RegionData::default();
	rd.key_range.set_hi(70).unwrap();
	rd.key_range.set_lo(60).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::E2, wmidi::Velocity::MAX), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::E2));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::E2, wmidi::Velocity::MIN), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::E2));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::E3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::E2));
	assert!(region.is_playing_note(wmidi::Note::E3));
	assert_eq!(region.gain, 0.24607849215698431397);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::E3, wmidi::Velocity::MIN), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::E2));
	assert!(region.is_playing_note(wmidi::Note::E3));
	pull_samples(&mut region, 2);
	assert!(!region.is_playing_note(wmidi::Note::E2));
	assert!(!region.is_playing_note(wmidi::Note::E3));
    }


    #[test]
    fn note_trigger_vel_range() {
	let mut rd = RegionData::default();
	rd.vel_range.set_hi(70).unwrap();
	rd.vel_range.set_lo(60).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(90).unwrap()), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
	let mut out_left = [0.0; 1];
	let mut out_right = [0.0; 1];
	region.process(&mut out_left, &mut out_right);
	assert!(region.is_playing_note(wmidi::Note::C3));
	assert_eq!(out_left[0], 0.24607849215698431397);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);
	pull_samples(&mut region, 2);
	assert!(!region.is_playing_note(wmidi::Note::C3));
    }


    #[test]
    fn region_trigger_cc() {
	let mut rd = RegionData::default();
	rd.push_on_lo_cc(64, 63).unwrap();
	rd.push_on_hi_cc(64, 127).unwrap();
	rd.push_on_hi_cc(42, 23).unwrap();

	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(wmidi::Channel::Ch1,
								wmidi::ControlNumber::try_from(23).unwrap(),
								wmidi::ControlValue::try_from(90).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(wmidi::Channel::Ch1,
								wmidi::ControlNumber::try_from(64).unwrap(),
								wmidi::ControlValue::try_from(23).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(wmidi::Channel::Ch1,
								wmidi::ControlNumber::try_from(42).unwrap(),
								wmidi::ControlValue::try_from(21).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(wmidi::Channel::Ch1,
								wmidi::ControlNumber::try_from(64).unwrap(),
								wmidi::ControlValue::try_from(90).unwrap()), 0.0);
	assert!(region.sample.is_playing());

    }


    #[test]
    fn note_trigger_release() {
	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
	assert_eq!(region.gain, 0.24607849215698431397);
    }

    #[test]
    fn trigger_release_rt_decay() {
    	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	rd.set_rt_decay(3.0).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, 1.0);

	let mut out_left = [0.0];
	let mut out_right = [0.0];

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.process(&mut out_left, &mut out_right);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, utils::dB_to_gain(-3.0));

	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	rd.set_rt_decay(3.0).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, 1.0);

	let mut out_left = [0.0, 0.0];
	let mut out_right = [0.0, 0.0];

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.process(&mut out_left, &mut out_right);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, utils::dB_to_gain(-6.0));

	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	rd.set_rt_decay(3.0).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, 1.0);

	let mut out_left = [0.0];
	let mut out_right = [0.0];

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.process(&mut out_left, &mut out_right);
	region.process(&mut out_left, &mut out_right);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert_eq!(region.gain, utils::dB_to_gain(-6.0));
    }

    #[test]
    fn note_trigger_release_sustain_pedal() {
    	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	let mut region = make_dummy_region(rd, 1.0, 2);

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(!region.sample.is_playing());

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(region.is_playing_note(wmidi::Note::C3));
	let (ol, _) = pull_samples(&mut region, 1);
	assert_eq!(ol[0], 0.24607849215698431397);


	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::Release);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));

    	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(region.is_playing_note(wmidi::Note::C3));
	let (ol, _) = pull_samples(&mut region, 1);
	assert_eq!(ol[0], 0.24607849215698431397);
    }

    #[test]
    fn note_trigger_release_key() {
	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::ReleaseKey);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
	let (ol, _) = pull_samples(&mut region, 1);
	assert_eq!(ol[0], 0.24607849215698431397);
    }

    #[test]
    fn note_trigger_release_key_vel_range() {
	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::ReleaseKey);
	rd.vel_range.set_hi(70).unwrap();
	rd.vel_range.set_lo(60).unwrap();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(90).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
	let (ol, _) = pull_samples(&mut region, 1);
	assert_eq!(ol[0], 0.24607849215698431397);
    }


    #[test]
    fn note_trigger_release_key_sustain_pedal() {
    	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::ReleaseKey);
	let mut region = make_dummy_region(rd, 1.0, 2);

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(!region.sample.is_playing());

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.is_playing_note(wmidi::Note::C3));

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(!region.sample.is_playing());


	let mut rd = RegionData::default();
	rd.set_trigger(Trigger::ReleaseKey);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);
	assert!(!region.sample.is_playing());

    	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(!region.sample.is_playing());
    }

    #[test]
    fn note_trigger_first() {
	let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::First);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

    	let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::First);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(!region.sample.is_playing());

        let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::First);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
    }

    #[test]
    fn note_trigger_legato() {
	let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::Legato);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(!region.sample.is_playing());

    	let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::Legato);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

        let mut rd = RegionData::default();
	rd.key_range.set_hi(60).unwrap();
	rd.key_range.set_lo(60).unwrap();
	rd.set_trigger(Trigger::Legato);
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(!region.sample.is_playing());
    }

    #[test]
    fn note_off_sustain_pedal() {
	let rd = RegionData::default();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	pull_samples(&mut region, 2);
	assert!(!region.sample.is_playing());
    }

    #[test]
    fn note_on_during_release() {
	let rd = RegionData::default();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);

	pull_samples(&mut region, 2);
	assert!(region.is_playing_note(wmidi::Note::C3));
    }

    #[test]
    fn note_on_off_during_release() {
	let rd = RegionData::default();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);

	pull_samples(&mut region, 2);
	assert!(!region.is_playing_note(wmidi::Note::C3));
    }

    #[test]
    fn note_on_off_detuned() {
	let mut rd = RegionData::default();
	rd.tune = 1.0;
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	pull_samples(&mut region, 2);
	assert!(!region.is_playing_note(wmidi::Note::C3));
    }

    #[test]
    fn note_remain_sustain_pedal() {
	let rd = RegionData::default();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	assert!(region.is_playing_note(wmidi::Note::C3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	pull_samples(&mut region, 2);
	assert!(!region.sample.is_playing());
    }

    #[test]
    fn note_off_polyphonic_sustain_pedal() {
	let rd = RegionData::default();
	let mut region = make_dummy_region(rd, 1.0, 2);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));

	// sustain pedal on
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(64).unwrap()
	), 0.0);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::D3,  wmidi::Velocity::MAX), 0.0);
	assert!(region.is_playing_note(wmidi::Note::C3));
	assert!(region.is_playing_note(wmidi::Note::D3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3,  wmidi::Velocity::MAX), 0.0);
	pull_samples(&mut region, 2);
	assert!(region.is_playing_note(wmidi::Note::C3));
	assert!(region.is_playing_note(wmidi::Note::D3));

	// sustain pedal off
	region.pass_midi_msg(&wmidi::MidiMessage::ControlChange(
	    wmidi::Channel::Ch1,
	    wmidi::ControlNumber::try_from(64).unwrap(),
	    wmidi::ControlValue::try_from(63).unwrap()
	), 0.0);

	pull_samples(&mut region, 2);
	assert!(!region.is_playing_note(wmidi::Note::C3));
	assert!(region.is_playing_note(wmidi::Note::D3));

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::D3,  wmidi::Velocity::MAX), 0.0);
	pull_samples(&mut region, 2);
	assert!(!region.sample.is_playing());
    }



    #[test]
    fn simple_note_on_off_process() {
	let sample = vec![0.1, -0.1,
			  0.2, -0.2,
			  0.3, -0.3,
			  0.4, -0.4,
			  0.5, -0.5];

	let mut engine = Engine::from_region_array(vec![(RegionData::default(), sample)], 1.0, 16);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);

	assert_eq!(out_left[0], 0.0);
	assert_eq!(out_right[0], -0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.1);
	assert_eq!(out_right[0], -0.1);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	engine.process(&mut out_left, &mut out_right);

	assert_eq!(out_left[0], 0.0);
	assert_eq!(out_right[0], 0.0);
    }


    #[test]
    fn note_on_off_adsr() {
	let mut sample = vec![];
	sample.resize(48, 1.0);
	let regions = parse_sfz_text("<region> ampeg_attack=2 ampeg_hold=3 ampeg_decay=4 ampeg_sustain=60 ampeg_release=5".to_string()).unwrap();

	let mut engine = Engine::from_region_array(vec![(regions[0].clone(), sample)], 1.0, 16);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));
	engine.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);

	let mut out_left: [f32; 4] = [0.0; 4];
	let mut out_right: [f32; 4] = [0.0; 4];

	engine.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(out.as_slice(), [0.6, 0.6, 0.6, 0.6]);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 8] = [0.0; 8];
	let mut out_right: [f32; 8] = [0.0; 8];

	engine.process(&mut out_left, &mut out_right);

	let rel: Vec<f32> = out_left.iter().map(|v| (v*10000.0).round()/10000.0).collect();
	assert_eq!(rel.as_slice(), [0.0727, 0.0147, 0.003, 0.0006, 0.0001, 0.0, 0.0, 0.0]);
    }


    #[test]
    fn note_on_velocity() {
	let sample = vec![1.0, 1.0];
	let mut region = Region::new(RegionData::default(), sample, 1.0, 16);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(63).unwrap()), 0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.24607849215698431397);
	assert_eq!(out_right[0], 0.24607849215698431397);
    }

    #[test]
    fn note_on_gain_veltrack() {
	let sample = vec![1.0, 1.0];
	let mut rd = RegionData::default();
	rd.set_amp_veltrack(0.0).unwrap();

	let mut region = Region::new(rd, sample.clone(), 1.0, 16);

	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_right[0], 1.0);


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_right[0], 1.0);


	let mut rd = RegionData::default();
	rd.set_amp_veltrack(-100.0).unwrap();

	let mut region = Region::new(rd, sample.clone(), 1.0, 16);


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN), 0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_right[0], 1.0);


	region.pass_midi_msg(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	region.pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], utils::dB_to_gain(-160.0));
	assert_eq!(out_right[0], utils::dB_to_gain(-160.0));

    }

    #[test]
    fn note_on_off_key_range() {
	let sample = vec![1.0, 1.0,
			  0.5, 0.5];

	let region = parse_sfz_text("<region> lokey=60 hikey=60".to_string()).unwrap()[0].clone();

	let mut engine = Engine::from_region_array(vec![(region.clone(), sample.clone())], 1.0, 16);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 0.0));
	assert!(f32_eq(out_right[0], 0.0));

	let mut engine = Engine::from_region_array(vec![(region.clone(), sample.clone())], 1.0, 16);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 1.0));
	assert!(f32_eq(out_right[0], 1.0));

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert!(f32_eq(out_left[0], 0.5));
	assert!(f32_eq(out_right[0], 0.5));
    }


    #[test]
    fn pitch_keytrack_frequency() {
	let samplerate = 48000.0;
	let nsamples = 96000;

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A4, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 880.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(0.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(0.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A4, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(-100.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(-100.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A4, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 220.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(1200.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.set_pitch_keytrack(1200.0).unwrap();

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::ASharp3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 880.0);
    }

    #[test]
    fn tune_frequency() {
	let samplerate = 48000.0;
	let nsamples = 96000;

	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.tune = 1.0;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::Ab3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.tune = -1.0;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::ASharp3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 440.0);


	let mut rd = RegionData::default();
	rd.pitch_keycenter = wmidi::Note::A3;
	rd.tune = 1.0;

	let sample_data = sampletests::make_test_sample_data(nsamples, samplerate, 440.0);
	let mut region = Region::new(rd, sample_data, samplerate, nsamples);

	region.note_on(wmidi::Note::A3, wmidi::Velocity::MAX);
	sampletests::assert_frequency(region.sample, samplerate, 466.16);
    }

    #[test]
    fn trigger_rand() {
	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	for i in 0..2 {
	    engine.regions[i].pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX), 0.0);
	}
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());

	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	for i in 0..2 {
	    engine.regions[i].pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX), 0.5);
	}
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());

	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	for i in 0..2 {
	    engine.regions[i].pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.0);
	}
	assert!(engine.regions[0].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[1].sample.is_playing());

	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	for i in 0..2 {
	    engine.regions[i].pass_midi_msg(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX), 0.5);
	}
	assert!(!engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));

	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX));
	assert!(!engine.regions[0].sample.is_playing() && !engine.regions[1].sample.is_playing());

	let region_text = "<region> key=c4 lorand=0.0 hirand=0.5 <region> key=c4 lorand=0.5 hirand=1.0".to_string();
	let mut engine = Engine::from_region_array(parse_sfz_text(region_text).unwrap().iter().map(|reg| (reg.clone(), Vec::new())).collect(), 1.0, 1);
	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));
	assert!(engine.regions[0].is_playing_note(wmidi::Note::C3) ^ engine.regions[1].is_playing_note(wmidi::Note::C3));
    }

    fn pull_samples_engine(engine: &mut Engine, nsamples: usize) {
	let mut out_left = Vec::new();
	out_left.resize(nsamples, 0.0);
	let mut out_right = Vec::new();
	out_right.resize(nsamples, 0.0);

	engine.process(&mut out_left, &mut out_right);
    }

    #[test]
    fn note_on_off_multiple_regions_key() {
	let region_text = "
<region> lokey=a3 hikey=a3 pitch_keycenter=57
<region> lokey=60 hikey=60 pitch_keycenter=60
<region> lokey=58 hikey=60 pitch_keycenter=60
<region> lokey=60 hikey=62 pitch_keycenter=61
".to_string();
	let regions = parse_sfz_text(region_text).unwrap();

	let mut engine = Engine::from_region_array(regions.iter().map(|reg| (reg.clone(), vec![1.0; 96])).collect(), 1.0, 1);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A1, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::A2));
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Db3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::A2));
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(engine.regions[3].is_playing_note(wmidi::Note::Db3));

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::A2));
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::Db3));

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::Db3));

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::Db3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[3].is_playing_note(wmidi::Note::Db3));

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::B2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::B2));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[3].is_playing_note(wmidi::Note::Db3));

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(engine.regions[2].is_playing_note(wmidi::Note::B2));
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::B2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());
    }

    #[test]
    fn note_on_off_multiple_regions_vel() {
	let region_text = "
<region> lovel=30 hivel=30 amp_veltrack=0
<region> lovel=50 hivel=50 amp_veltrack=0
<region> lovel=40 hivel=50 amp_veltrack=0
<region> lovel=50 hivel=60 amp_veltrack=0
".to_string();

	let regions = parse_sfz_text(region_text).unwrap();

	let mut engine = Engine::from_region_array(regions.iter().map(|reg| (reg.clone(), vec![1.0; 96])).collect(), 1.0, 1);
	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(20).unwrap()));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(30).unwrap()));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(55).unwrap()));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(50).unwrap()));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[1].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(engine.regions[3].is_playing_note(wmidi::Note::C3));

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::try_from(45).unwrap()));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(engine.regions[2].is_playing_note(wmidi::Note::C3));
	assert!(!engine.regions[3].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MIN));
	pull_samples_engine(&mut engine, 1);
	assert!(!engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());

    }


    #[test]
    fn region_group() {
	let region_text = "
<region> key=a3
<region> key=b3 group=1
<region> key=c4 group=2
<region> key=d4 off_by=2
<region> key=e4 group=1
".to_string();

	let regions = parse_sfz_text(region_text).unwrap();

	let mut engine = Engine::from_region_array(regions.iter().map(|reg| (reg.clone(), vec![1.0; 96])).collect(), 1.0, 1);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());
	assert!(!engine.regions[4].sample.is_playing());

    	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::D3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(engine.regions[3].sample.is_playing());
	assert!(!engine.regions[4].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::B2, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].sample.is_playing());
	assert!(!engine.regions[2].sample.is_playing());
	assert!(engine.regions[3].sample.is_playing());
	assert!(!engine.regions[4].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].sample.is_playing());
	assert!(engine.regions[1].sample.is_playing());
	assert!(engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());
	assert!(!engine.regions[4].sample.is_playing());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::E3, wmidi::Velocity::MAX));
	pull_samples_engine(&mut engine, 1);
	assert!(engine.regions[0].sample.is_playing());
	assert!(!engine.regions[1].sample.is_playing());
	assert!(engine.regions[2].sample.is_playing());
	assert!(!engine.regions[3].sample.is_playing());
	assert!(engine.regions[4].sample.is_playing());
    }

    #[test]
    fn test_real_sample() {
	let mut snd = sndfile::OpenOptions::ReadOnly(sndfile::ReadOptions::Auto).from_path("assets/gmidi-grand-piano-C4.flac").unwrap();
	let sample = snd.read_all_to_vec().unwrap();

	let mut reference = [vec![0.0f32; 2048], sample.clone()].concat();
	reference.resize(2 * 30 * 48000 + 1024, 0.0f32);

	let mut out_left = Vec::new();
	out_left.resize(30 * 48000 + 1024, 0.0);
	let mut out_right = Vec::new();
	out_right.resize(30 * 48000 + 1024, 0.0);

	let goal = (30 * 48000) / 1024;

	let mut engine = Engine::new("assets/simple-test-instrument.sfz".to_string(), 48000.0, 1024).unwrap();

	engine.process(&mut out_left, &mut out_right);
	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	for i in 1..goal {
	    engine.process(&mut out_left[i*1024..(i+1)*1024], &mut out_right[i*1024..(i+1)*1024]);
	}

	let mut result = Vec::with_capacity(reference.len());
	for (l, r) in Iterator::zip(out_left.iter(), out_right.iter()) {
	    result.push(l);
	    result.push(r);
	}

	assert!(!Iterator::zip(reference.iter(), result.iter()).any( |(a, b)| a != *b ));
    }
}
