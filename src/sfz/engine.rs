

use super::errors::*;


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
struct RegionState {
    active: bool,
    position: u32,
}

impl Default for RegionState {
    fn default() -> Self {
	RegionState {
	    active: false,
	    position: 0
	}
    }
}

#[derive(Clone)]
pub struct Region {
    pub(super) key_range: NoteRange,
    pub(super) vel_range: VelRange,

    pitch_keycenter: wmidi::Note,

    pitch_keytrack: f32,

    amp_veltrack: f32,
    ampeg_release: f32,

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

    sample_data: Vec<(f32, f32)>,
    state: RegionState

}

impl Default for Region {
    fn default() -> Self {
	Region {
	    key_range: Default::default(),
	    vel_range: Default::default(),

	    pitch_keycenter: wmidi::Note::C4,

	    pitch_keytrack: 100.0,

	    ampeg_release: Default::default(),
	    amp_veltrack: 1.0,

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

	    sample_data: Vec::new(),
	    state: Default::default()
	}
    }
}

impl Region {
    pub(super) fn set_amp_veltrack(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    v if v >= -100 && v <= 100 => {
		self.amp_veltrack = (v as f32) / 100.0;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("amp_veltrack", -100, 100, v))
	}
    }

    pub(super) fn set_ampeg_release(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v >= 0.0 && v <= 100.0 => {
		self.ampeg_release = v as f32;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("ampeg_release", 0.0, 100.0, v))
	}
    }

    pub(super) fn set_pitch_keycenter(&mut self, v: u32) -> Result<(), RangeError> {
	match v {
	    v if v <= 127 => {
		self.pitch_keycenter = unsafe { wmidi::Note::from_u8_unchecked(v as u8) };
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("pitch_center", 0, 127, v))
	}
    }

    pub(super) fn set_pitch_keytrack(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v >= -1200.0 && v <= 1200.0 => {
		self.pitch_keytrack = v;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("pitch_keytrack", -1200.0, 1200.0, v))
	}
    }

    pub(super) fn set_sample(&mut self, v: &str) {
	self.sample = v.to_string();
    }

    pub(super) fn set_rt_decay(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v >= 0.0 && v <= 200.0 => {
		self.rt_decay = v;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("rt_decay", 0.0, 200.0, v))
	}
    }

    pub(super) fn set_tune(&mut self, v: i32) -> Result<(), RangeError> {
	match v {
	    v if v >= -100 && v <= 100 => {
		self.tune = v as i8;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("tune", -100, 100, v))
	}
    }

    pub(super) fn set_volume(&mut self, v: f32) -> Result<(), RangeError> {
	match v {
	    v if v >= -144.6 && v <= 6.0 => {
		self.volume = v;
		Ok(())
	    }
	    _ => Err(RangeError::out_of_range("volume", -144.6, 6.0, v))
	}
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

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]) {
	if !self.state.active {
	    return;
	}

	let mut position = self.state.position as usize;

	for (l, r) in Iterator::zip(out_left.iter_mut(), out_right.iter_mut()) {
	    if position >= self.sample_data.len() {
		position = 0;
		self.state.active = false;
		break;
	    }
	    let (sl, sr) = self.sample_data[position];
	    *l += sl;
	    *r += sr;

	    position += 1;
	}

	self.state.position = position as u32;
    }
}


pub struct Engine {
    pub(super) regions: Vec<Region>
}



#[cfg(test)]
mod tests {

    use super::*;
    use super::super::parser::parse_sfz_text;

    #[test]
    fn region_data_default() {
	let rd: Region = Default::default();

	assert_eq!(rd.key_range.hi, Some(wmidi::Note::HIGHEST_NOTE));
	assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	assert_eq!(rd.vel_range.hi, 127);
	assert_eq!(rd.vel_range.lo, 0);

	assert_eq!(rd.amp_veltrack, 1.0);
	assert_eq!(rd.ampeg_release, 0.0);
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
	let engine = parse_sfz_text("<region> hikey=42 lokey=23".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 1);
	match &engine.regions.get(0) {
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
	let engine = parse_sfz_text("<region> hikey=c#3 lokey=ab2 <region> hikey=c3 lokey=a2".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 2);
	match &engine.regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::Db2));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::GSharp1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(1) {
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
	let engine = parse_sfz_text("<group> hivel=42 lovel=23".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 0);
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
	let engine = parse_sfz_text("<region> hivel=42 lovel=23 // foo".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 1);
	match &engine.regions.get(0) {
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
	let engine = parse_sfz_text("<region> hivel=42 lovel=23 \n hikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 1);
	match &engine.regions.get(0) {
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
	let engine = parse_sfz_text("<region> hivel=42 lovel=23 // foo bar foo\nhikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(engine.regions.len(), 1);
	match &engine.regions.get(0) {
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

	let engine = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(engine.regions.len(), 2)
    }

    #[test]
    fn parse_regions_inheriting_group_data() {
	let s = "
<group> hivel=42
<region> lovel=23
<region> lovel=21
";
	let engine = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(engine.regions.len(), 2);
	match &engine.regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(1) {
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
	let engine = parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(engine.regions.len(), 6);
	match &engine.regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(2) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(3) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(4) {
	    Some(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &engine.regions.get(5) {
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
	let engine = parse_sfz_text(s.to_string()).unwrap();

	assert_eq!(engine.regions.len(), 12);
	match &engine.regions.get(0) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		assert_eq!(rd.ampeg_release, 1.0);
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
	match &engine.regions.get(1) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		assert_eq!(rd.ampeg_release, 1.0);
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
	match &engine.regions.get(2) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		assert_eq!(rd.ampeg_release, 5.0);
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
	match &engine.regions.get(3) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.73);
		assert_eq!(rd.ampeg_release, 5.0);
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
	match &engine.regions.get(4) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.94);
		assert_eq!(rd.ampeg_release, 0.0);
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
	match &engine.regions.get(5) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.94);
		assert_eq!(rd.ampeg_release, 0.0);
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
	match &engine.regions.get(6) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.82);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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
	match &engine.regions.get(7) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 0.82);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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
	match &engine.regions.get(8) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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
	match &engine.regions.get(9) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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
	match &engine.regions.get(10) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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
	match &engine.regions.get(11) {
	    Some(rd) => {
		assert_eq!(rd.amp_veltrack, 1.0);
		assert_eq!(rd.ampeg_release, 0.0);
		assert_eq!(rd.pitch_keycenter, wmidi::Note::C4);
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

    #[test]
    fn simple_region_process() {
	let sample = vec![(1.0, 0.5), (0.5, 1.0), (1.0, 0.5)];

	let mut region = Region::default();
	region.state.active = true;
	region.sample_data = sample.clone();

	let mut out_left: [f32; 2] = [0.0, 0.0];
	let mut out_right: [f32; 2] = [0.0, 0.0];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(region.state.active, true);
	assert_eq!(region.state.position, 2);
	assert_eq!(out_left[0], 1.0);
	assert_eq!(out_left[1], 0.5);

	assert_eq!(out_right[0], 0.5);
	assert_eq!(out_right[1], 1.0);

	let mut out_left: [f32; 2] = [-0.5, -0.2];
	let mut out_right: [f32; 2] = [-0.2, -0.5];

	region.process(&mut out_left, &mut out_right);
	assert_eq!(region.state.active, false);
	assert_eq!(region.state.position, 0);
	assert_eq!(out_left[0], 0.5);
	assert_eq!(out_left[1], -0.2);

	assert_eq!(out_right[0], 0.3);
	assert_eq!(out_right[1], -0.5);
    }
}
