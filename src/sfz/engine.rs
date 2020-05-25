use std::fmt;
use std::slice;

use crate::errors::*;


use crate::engine;
use crate::adsr_eg as envelopes;
use crate::utils;

#[derive(Clone, Copy)]
pub(super) struct VelRange {
    lo: i8,
    hi: i8
}

impl VelRange {
    pub(super) fn set_hi(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    v if v < 0 && v > 127 => Err(RangeError::out_of_range("hivel", 0, 127, v)),
	    v if (v as i8) < self.lo => Err(RangeError::flipped_range("hivel", v, self.lo as i32)),
	    _ => {
		self.hi = v as i8;
		Ok(())
	    }
	}
    }

    pub(super) fn set_lo(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    v if v < 0 && v > 127 => Err(RangeError::out_of_range("lovel", 0, 127, v)),
	    v if (v as i8) > self.hi => Err(RangeError::flipped_range("lovel", v, self.lo as i32)),
	    _ => {
		self.lo = v as i8;
		Ok(())
	    }
	}
    }
}


impl Default for VelRange {
    fn default() -> Self {
	VelRange { hi: 127, lo: 0 }
    }
}

#[derive(Clone, Copy)]
pub(super) struct NoteRange {
    lo: Option<wmidi::Note>,
    hi: Option<wmidi::Note>
}

impl NoteRange {
    pub(super) fn set_hi(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    -1 => {
		self.hi = None;
		Ok(())
	    }
	    v if v < 0 && v > 127 => Err(RangeError::out_of_range("hikey", -1, 127, v)),
	    _ => {
		let note = unsafe { wmidi::Note::from_u8_unchecked(v as u8) };
		if self.lo.map_or(false, |n| note < n) {
		    return Err(RangeError::flipped_range("hikey", v, u8::from(note) as i32));
		}
		self.hi = Some(note);
		Ok(())
	    }
	}
    }

    pub(super) fn set_lo(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    -1 => {
		self.lo = None;
		Ok(())
	    }
	    v if v > 127 => Err(RangeError::out_of_range("lokey", -1, 127, v)),
	    _ => {
		let note = unsafe { wmidi::Note::from_u8_unchecked(v as u8) };
		if self.hi.map_or(false, |n| note > n) {
		    return Err(RangeError::flipped_range("lokey", v, u8::from(note) as i32));
		}
		self.lo = Some(note);
		Ok(())
	    }
	}
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

    pitch_keytrack: f32,

    amp_veltrack: f32,

    volume: f32,

    sample: String,
    rt_decay: f32,

    tune: i8,

    trigger: Trigger,

    group: u32,
    off_by: u32,

    on_lo_cc: (u32, i32),
    on_hi_cc: (u32, i32),

    pub(super) random_range: RandomRange,
}


impl Default for RegionData {
    fn default() -> Self {
	RegionData {
	    key_range: Default::default(),
	    vel_range: Default::default(),

	    pitch_keycenter: wmidi::Note::C3,

	    pitch_keytrack: 100.0,

	    amp_veltrack: 1.0,

	    ampeg: Default::default(),

	    volume: Default::default(),
	    sample: Default::default(),
	    rt_decay: Default::default(),
	    tune: Default::default(),
	    trigger: Default::default(),

	    group:  Default::default(),
	    off_by:  Default::default(),

	    on_lo_cc: (0, 0),
	    on_hi_cc: (0, 0),

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
	self.pitch_keytrack = range_check(v, -1200.0, 1200.0, "pitch_keytrack")?;
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
	self.tune = range_check(v, -100, 100, "tune")? as i8;
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

    pub(super) fn set_on_lo_cc(&mut self, channel: u32, v: i32) {
	self.on_lo_cc = (channel, v);
    }

    pub(super) fn set_on_hi_cc(&mut self, channel: u32, v: i32) {
	self.on_hi_cc = (channel, v);
    }
}


#[derive(Clone)]
struct RegionState {
    sample_position: Option<usize>,
}

impl Default for RegionState {
    fn default() -> Self {
	RegionState {
	    sample_position: None,
	}
    }
}


pub(super) struct Region {
    params: RegionData,

    sample_data: Vec<f32>,
    state: RegionState,

    amp_envelope: envelopes::ADSREnvelope,

    velocity_amp: f32,

    samplerate: f32,
    real_sample_length: usize,
    max_block_length: usize
}

impl Region {
    fn new(params: RegionData, samplerate: f32, max_block_length: usize) -> Region {

	let amp_envelope = envelopes::ADSREnvelope::new(&params.ampeg, samplerate, max_block_length);

	Region {
	    params: params,

	    sample_data: Vec::new(),
	    state: Default::default(),

	    velocity_amp: 1.0,

	    amp_envelope: amp_envelope,

	    samplerate: samplerate,
	    max_block_length: max_block_length,
	    real_sample_length: 0,
	}
    }

    fn set_sample_data(&mut self,  mut sample_data: Vec<f32>) {
	let frames = sample_data.len() / 2;

	let reserve_frames = ((frames / self.max_block_length) + 2) * self.max_block_length;

	sample_data.resize(reserve_frames * 2, 0.0);
	self.sample_data = sample_data;
	self.real_sample_length = frames;
    }

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
	let mut position = match self.state.sample_position {
	    Some(p) => p,
	    None => return
	};

	let gain = utils::dB_to_gain(self.params.volume) * self.velocity_amp;

	let (envelope, mut env_position) = self.amp_envelope.active_envelope();

	for (l, r) in Iterator::zip(out_left.iter_mut(), out_right.iter_mut()) {
	    let sl = self.sample_data[position];
	    let sr = self.sample_data[position+1];

	    *l += sl * gain * envelope[env_position];
	    *r += sr * gain * envelope[env_position];

	    position += 2;
	    env_position += 1;
	}

	self.amp_envelope.update(env_position);

	self.state.sample_position = if position < self.real_sample_length * 2 {
	    Some(position)
	} else {
	    None
	}
    }

    fn is_active(&self) -> bool {
	self.state.sample_position.is_some()
    }

    fn note_on(&mut self, velocity: u8) {
	if self.is_active() {
	    return;
	}

	self.velocity_amp = velocity as f32 / 127.0;

	self.state.sample_position = Some(0);

	self.amp_envelope.note_on();

    }

    fn note_off(&mut self) {
	if self.is_active() {
	    self.amp_envelope.note_off();
	}
    }

    fn handle_note_on(&mut self, note: wmidi::Note, velocity: wmidi::Velocity) {
	if !self.params.key_range.covering(note) {
	    return;
	}

	self.note_on(u8::from(velocity));
    }

    fn handle_note_off(&mut self, note: wmidi::Note) {
	if self.params.key_range.covering(note) {
	    self.note_off();
	}
    }

    fn pass_midi_msg(&mut self, midi_msg: &wmidi::MidiMessage) {
	match midi_msg {
	    wmidi::MidiMessage::NoteOn(_ch, note, vel) => self.handle_note_on(*note, *vel),
	    wmidi::MidiMessage::NoteOff(_ch, note, _vel) => self.handle_note_off(*note),
	    _ => {}
	}
    }
}


pub struct Engine {
    pub(super) regions: Vec<Region>,
    samplerate: f32,
    max_block_length: usize
}

impl Engine {
    fn new(reg_data: Vec<RegionData>, samplerate: f32, max_block_length: usize) -> Engine {
	Engine {
	    regions: reg_data.iter().map(|rd| Region::new(rd.clone(), samplerate, max_block_length)).collect(),
	    samplerate: samplerate,
	    max_block_length: 1
	}
    }
}

impl engine::EngineTrait for Engine {
    fn midi_event(&mut self, midi_msg: &wmidi::MidiMessage) {
	for r in &mut self.regions {
	    r.pass_midi_msg(midi_msg);
	}
    }

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
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

    use std::f32::consts::PI;

    #[test]
    fn region_data_default() {
	let rd: RegionData = Default::default();

	assert_eq!(rd.key_range.hi, Some(wmidi::Note::HIGHEST_NOTE));
	assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	assert_eq!(rd.vel_range.hi, 127);
	assert_eq!(rd.vel_range.lo, 0);

	assert_eq!(rd.amp_veltrack, 1.0);
/* FIXME: How to test this?
	let mut env = envelopes::ADSREnvelope::new(&rd.ampeg, 1.0, 4);
	let (sustain_env, _) = env.active_envelope();
	assert_eq!(*sustain_env.as_slice(), [1.0; 4]);
*/
	assert_eq!(rd.tune, 0)
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
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
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
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::C2));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::A1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
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
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23);
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
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23);
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
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23);
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
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 21)
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
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(2) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(3) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(4) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &regions.get(5) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 23);
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
		assert_eq!(rd.tune, 10);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.vel_range.hi, 26);
		assert_eq!(rd.vel_range.lo, 1);
		assert_eq!(rd.sample, "48khz24bit\\A0v1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 10);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.vel_range.hi, 34);
		assert_eq!(rd.vel_range.lo, 27);
		assert_eq!(rd.sample, "48khz24bit\\A0v2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, -13);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G5));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::F5));
		assert_eq!(rd.vel_range.hi, 26);
		assert_eq!(rd.vel_range.lo, 1);
		assert_eq!(rd.sample, "48khz24bit\\F#6v1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, -13);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G5));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::F5));
		assert_eq!(rd.vel_range.hi, 34);
		assert_eq!(rd.vel_range.lo, 27);
		assert_eq!(rd.sample, "48khz24bit\\F#6v2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::BbMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AbMinus1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 45);
		assert_eq!(rd.sample, "48khz24bit\\harmLA0.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 6.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::Db0));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 45);
		assert_eq!(rd.sample, "48khz24bit\\harmLC1.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 6.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::AMinus1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\rel1.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 2.0);
		assert_eq!(rd.pitch_keytrack, 0.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::ASharpMinus1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::ASharpMinus1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\rel2.wav");
		assert_eq!(rd.trigger, Trigger::Release);
		assert_eq!(rd.rt_decay, 2.0);
		assert_eq!(rd.pitch_keytrack, 0.0);
		assert_eq!(rd.group, 0);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (0, 0));
		assert_eq!(rd.on_hi_cc, (0, 0));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\pedalD1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 1);
		assert_eq!(rd.off_by, 2);
		assert_eq!(rd.on_lo_cc, (64, 126));
		assert_eq!(rd.on_hi_cc, (64, 127));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\pedalD2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 1);
		assert_eq!(rd.off_by, 2);
		assert_eq!(rd.on_lo_cc, (64, 126));
		assert_eq!(rd.on_hi_cc, (64, 127));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\pedalU1.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 2);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (64, 0));
		assert_eq!(rd.on_hi_cc, (64, 1));
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
		assert_eq!(rd.tune, 0);
		assert_eq!(rd.key_range.hi, None);
		assert_eq!(rd.key_range.lo, None);
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.sample, "48khz24bit\\pedalU2.wav");
		assert_eq!(rd.trigger, Trigger::Attack);
		assert_eq!(rd.rt_decay, 0.0);
		assert_eq!(rd.pitch_keytrack, 100.0);
		assert_eq!(rd.group, 2);
		assert_eq!(rd.off_by, 0);
		assert_eq!(rd.on_lo_cc, (64, 0));
		assert_eq!(rd.on_hi_cc, (64, 1));
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
    fn region_sample_data() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let mut region = Region::new(RegionData::default(), 1.0, 16);

	region.set_sample_data(sample);

	assert_eq!(region.sample_data.len(), 64);
    }

    #[test]
    fn simple_region_process() {
	let sample = vec![1.0, 0.5,
			  0.5, 1.0,
			  1.0, 0.5];

	let mut region = Region::new(RegionData::default(), 1.0, 8);
	region.set_sample_data(sample);

	region.note_on(127);

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_left[1], 0.5);

	assert_eq!(out_right[0], 0.5);
	assert_eq!(out_right[1], 1.0);

	assert!(region.is_active());

	let mut out_left: [f32; 2] = [-0.5, -0.2];
	let mut out_right: [f32; 2] = [-0.2, -0.5];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_left[1], -0.2);

	assert_eq!(out_right[0], 0.3);
	assert_eq!(out_right[1], -0.5);

	assert!(!region.is_active());
    }

    #[test]
    fn region_volume_process() {
	let sample = vec![1.0, 1.0];

	let mut region_data = RegionData::default();
	region_data.set_volume(-20.0).unwrap();

	let mut region = Region::new(region_data, 1.0, 8);
	region.set_sample_data(sample.clone());

	region.note_on(127);

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

	let mut region = Region::new(regions.get(0).unwrap().clone(), 1.0, 16);
	region.set_sample_data(sample.clone());
	region.note_on(127);

	let mut out_left: [f32; 12] = [0.0; 12];
	let mut out_right: [f32; 12] = [0.0; 12];

	region.process(&mut out_left, &mut out_right);

	let out: Vec<f32> = out_left.iter().map(|v| (v*100.0).round()/100.0).collect();
	assert_eq!(out.as_slice(), [0.0, 0.5, 1.0, 1.0, 1.0, 0.65, 0.61, 0.6, 0.6, 0.6, 0.6, 0.6]);
    }

  #[test]
    fn region_amp_envelope_process_sustain() {
	let mut sample = vec![1.0; 96];

	let regions = parse_sfz_text("<region> ampeg_attack=2 ampeg_hold=3 ampeg_decay=4 ampeg_sustain=60 ampeg_release=5".to_string()).unwrap();

	let mut region = Region::new(regions.get(0).unwrap().clone(), 1.0, 12);
	region.set_sample_data(sample.clone());
	region.note_on(127);

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
    fn simple_engine_process() {
	let sample1 = vec![1.0, 0.5,
			   0.5, 1.0,
			   1.0, 0.5];
	let sample2 = vec![-0.5, 0.5,
			   -0.5, -0.5,
			   0.0, 0.5];

	let mut engine = Engine::new(vec![RegionData::default(), RegionData::default()], 1.0, 16);

	engine.regions[0].set_sample_data(sample1);
	engine.regions[0].note_on(127);
	engine.regions[1].set_sample_data(sample2);
	engine.regions[1].note_on(127);

	let mut out_left: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
	let mut out_right: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

	engine.process(&mut out_left, &mut out_right);

	assert!(!engine.regions[0].is_active());
	assert!(!engine.regions[1].is_active());

	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_left[1], 0.0);
	assert_eq!(out_left[2], 1.0);

	assert_eq!(out_right[0], 1.0);
	assert_eq!(out_right[1], 0.5);
	assert_eq!(out_right[2], 1.0);
    }



    #[test]
    fn simple_note_on_off() {
	let sample = vec![0.1, -0.1,
			  0.2, -0.2,
			  0.3, -0.3,
			  0.4, -0.4,
			  0.5, -0.5];

	let mut engine = Engine::new(vec![RegionData::default()], 1.0, 16);

	engine.regions[0].set_sample_data(sample.clone());

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

	let mut engine = Engine::new(regions, 1.0, 16);

	engine.regions[0].set_sample_data(sample.clone());

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
	assert_eq!(rel.as_slice(), [0.1211, 0.0245, 0.0049, 0.0010, 0.0002, 0.0, 0.0, 0.0]);
    }


    #[test]
    fn note_on_velocity() {
	let mut sample = vec![1.0, 1.0];

	let mut engine = Engine::new(vec![RegionData::default()], 1.0, 16);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1,
						      wmidi::Note::C3,
						      unsafe { wmidi::Velocity::from_unchecked(63) }));

	engine.regions[0].set_sample_data(sample.clone());

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.49606299212598425197);
	assert_eq!(out_right[0], 0.49606299212598425197);
    }

    #[test]
    fn note_on_off_key_range() {
	let mut sample = vec![1.0, 1.0,
			      0.5, 0.5];

	let regions = parse_sfz_text("<region> lokey=60 hikey=60".to_string()).unwrap();

	let mut engine = Engine::new(regions, 1.0, 16);

	engine.regions[0].set_sample_data(sample.clone());

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.0);
	assert_eq!(out_right[0], 0.0);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	engine.regions[0].set_sample_data(sample.clone());

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_right[0], 1.0);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A3, wmidi::Velocity::MAX));

	engine.regions[0].set_sample_data(sample.clone());

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_right[0], 0.5);
    }

//    #[test]
    fn note_on_off_mutiple_regions() {
	let mut sample1 = vec![ 0.1; 12];
	let mut sample2 = vec![ 0.2; 12];
	let mut sample3 = vec![ 0.3; 12];
	let mut sample4 = vec![-0.8; 12];

	let region_text = "
<region> lokey=a3 hikey=a3
<region> lokey=60 hikey=60
<region> lokey=58 hikey=60
<region> lokey=60 hikey=62
".to_string();
	let regions = parse_sfz_text(region_text).unwrap();

	let mut engine = Engine::new(regions, 1.0, 1);

	engine.regions[0].set_sample_data(sample1.clone());
	engine.regions[1].set_sample_data(sample2.clone());
	engine.regions[2].set_sample_data(sample3.clone());
	engine.regions[3].set_sample_data(sample4.clone());


	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A1, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.0);
	assert_eq!(out_right[0], 0.0);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A2, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.1);
	assert_eq!(out_right[0], 0.1);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::Db3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], -0.7);
	assert_eq!(out_right[0], -0.7);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], -0.5);
	assert_eq!(out_right[0], -0.5);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::A2, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], -0.6);
	assert_eq!(out_right[0], -0.6);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::Db3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.2);
	assert_eq!(out_right[0], 0.2);

	engine.midi_event(&wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::B2, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_right[0], 0.5);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::C3, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.3);
	assert_eq!(out_right[0], 0.3);

	engine.midi_event(&wmidi::MidiMessage::NoteOff(wmidi::Channel::Ch1, wmidi::Note::B2, wmidi::Velocity::MAX));

	let mut out_left: [f32; 1] = [0.0];
	let mut out_right: [f32; 1] = [0.0];

	engine.process(&mut out_left, &mut out_right);
	assert_eq!(out_left[0], 0.0);
	assert_eq!(out_right[0], 0.0);
    }
    /*


//    #[test]
    fn region_group() {
	let sample1 = vec![(0.1, -0.1), (0.1, -0.1), (0.1, -0.1), (0.1, -0.1), (0.1, -0.1)];
	let sample2 = vec![(0.2, -0.2), (0.2, -0.2), (0.2, -0.2), (0.2, -0.2), (0.2, -0.2)];
	let sample3 = vec![(0.3, -0.3), (0.3, -0.3), (0.3, -0.3), (0.3, -0.3), (0.3, -0.3)];

	let mut engine = Engine { regions: Vec::new() };

	let mut region = Region::default();
	region.sample_data = sample1.clone();
	region.set_group(1);

	engine.regions.push(region);

	let mut region = Region::default();
	region.sample_data = sample2.clone();
	region.set_group(1);

	engine.regions.push(region);

	let mut region = Region::default();
	region.sample_data = sample3.clone();

	engine.regions.push(region);

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	engine.regions[0].state.position = Some(0);
	engine.regions[2].state.position = Some(0);
	engine.process(&mut out_left, &mut out_right);

	assert_eq!(out_left[0], 0.4);
	assert_eq!(out_right[0], -0.4);
	assert_eq!(out_left[1], 0.4);
	assert_eq!(out_right[1], -0.4);

	assert_eq!(engine.regions[0].state.position, Some(2));

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	engine.regions[1].state.position = Some(0);
	engine.process(&mut out_left, &mut out_right);

	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_right[0], -0.5);
	assert_eq!(out_left[1], 0.5);
	assert_eq!(out_right[1], -0.5);
	assert_eq!(engine.regions[0].state.position, None);

    }

    fn make_test_sample(nsamples: u32, samplerate: f32, freq: f32) -> Vec<f32> {
	let omega = freq/samplerate * 2.0*PI;
	(0..nsamples).map(|t| ((omega * t as f32).sin())).collect()
    }

    fn test_calc_frequency(samplerate: f32, sample: Vec<f32>, test_freq: f32) -> bool {
	let (zeros, _) = sample.iter().fold((0, 0.0), |(n, last), s| {
	    if last * s < 0.0 {
		(n + 1, *s)
	    } else {
		(n, *s)
	    }
	});

	let zeros = zeros as f32;
	let to_freq = samplerate/(sample.len() as f32);
	zeros * to_freq < test_freq && (zeros + 1.0) * to_freq > test_freq
    }

*/
}
