

use super::errors::*;

pub(super) enum Tag {
    Group(RegionData),
    Region(RegionData)
}


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
pub struct RegionData {
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

    pub(super) random_range: RandomRange
}

impl Default for RegionData {
    fn default() -> Self {
	RegionData {
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

	    random_range: Default::default()
	}
    }
}

impl RegionData {
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
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::super::parser::Parser;

    #[test]
    fn region_data_default() {
	let rd: RegionData = Default::default();

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
	let mut parser = Parser::new();
	match parser.parse_sfz_text("".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "General parser error: Expecting <> tag in sfz file"),
	    _ => panic!("Expected error message")
	}
    }

    #[test]
    fn parse_sfz_hikey_lokey_region_line() {
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<region> hikey=42 lokey=23".to_string()).unwrap();
	assert_eq!(tags.len(), 1);
	match &tags[0] {
	    Tag::Region(rd) => {
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
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<region> hikey=c#3 lokey=ab2 <region> hikey=c3 lokey=a2".to_string()).unwrap();
	assert_eq!(tags.len(), 2);
	match &tags[0] {
	    Tag::Region(rd) => {
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::Db2));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::GSharp1));
		assert_eq!(rd.vel_range.hi, 127);
		assert_eq!(rd.vel_range.lo, 0);
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[1] {
	    Tag::Region(rd) => {
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
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<group> hivel=42 lovel=23".to_string()).unwrap();
	assert_eq!(tags.len(), 0);
    }

    #[test]
    fn parse_sfz_invalid_header_line() {
	let mut parser = Parser::new();
	match parser.parse_sfz_text("<foo> hikey=42 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Unknown key: foo"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_invalid_opcode_line() {
	let mut parser = Parser::new();
	match parser.parse_sfz_text("<region> foo=42 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Unknown key: foo"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_invalid_non_int_value_line() {
	let mut parser = Parser::new();
	match parser.parse_sfz_text("<region> hikey=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "Invalid key: aa"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_amp_veltrack() {
	let mut parser = Parser::new();
	match parser.parse_sfz_text("<region> amp_veltrack=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "amp_veltrack out of range: -100 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parser.parse_sfz_text("<region> amp_veltrack=-105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "amp_veltrack out of range: -100 <= -105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_out_of_range_ampeg_release() {
	let mut parser = Parser::new();
	match parser.parse_sfz_text("<region> ampeg_release=105 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_release out of range: 0 <= 105 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parser.parse_sfz_text("<region> ampeg_release=-20 lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e), "ampeg_release out of range: 0 <= -20 <= 100"),
	    _ => panic!("Not seen expected error")
	}
	match parser.parse_sfz_text("<region> ampeg_release=aa lokey=23".to_string()) {
	    Err(e) => assert_eq!(format!("{}", e),  "invalid float literal"),
	    _ => panic!("Not seen expected error")
	}
    }

    #[test]
    fn parse_sfz_comment_in_line() {
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<region> hivel=42 lovel=23 // foo".to_string()).unwrap();
	assert_eq!(tags.len(), 1);
	match &tags[0] {
	    Tag::Region(rd) => {
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
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<region> hivel=42 lovel=23 \n hikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(tags.len(), 1);
	match &tags[0] {
	    Tag::Region(rd) => {
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
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text("<region> hivel=42 lovel=23 // foo bar foo\nhikey=43 lokey=24".to_string()).unwrap();
	assert_eq!(tags.len(), 1);
	match &tags[0] {
	    Tag::Region(rd) => {
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
	let mut parser = Parser::new();
	let s = "<region> hivel=41 lovel=22 <region> hikey=42 lokey=23";

	let tags = parser.parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(tags.len(), 2)
    }

    #[test]
    fn parse_regions_inheriting_group_data() {
	let mut parser = Parser::new();
	let s = "
<group> hivel=42
<region> lovel=23
<region> lovel=21
";
	let tags = parser.parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(tags.len(), 2);
	match &tags[0] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 23)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[1] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 21)
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
    }

    #[test]
    fn parse_regions_inheriting_group_data_2groups() {
	let mut parser = Parser::new();
	let s = "
<group> hivel=42 hikey=41
<region> lokey=23
<region> lovel=21
<group> hikey=42 hivel=41
<region> lokey=23
<region> lovel=21
<region> hikey=43 hivel=42 lokey=23
";
	let tags = parser.parse_sfz_text(s.to_string()).unwrap();
	assert_eq!(tags.len(), 5);
	match &tags[0] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[1] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::F1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[2] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[3] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 41);
		assert_eq!(rd.vel_range.lo, 21);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::FSharp1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::LOWEST_NOTE));
	    }
	    _ => panic!("Expected region, got somthing different.")
	}
	match &tags[4] {
	    Tag::Region(rd) => {
		assert_eq!(rd.vel_range.hi, 42);
		assert_eq!(rd.vel_range.lo, 0);
		assert_eq!(rd.key_range.hi, Some(wmidi::Note::G1));
		assert_eq!(rd.key_range.lo, Some(wmidi::Note::BMinus1));
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
	let mut parser = Parser::new();
	let tags = parser.parse_sfz_text(s.to_string()).unwrap();

	assert_eq!(tags.len(), 12);
	match &tags[0] {
	    Tag::Region(rd) => {
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
	match &tags[1] {
	    Tag::Region(rd) => {
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
	match &tags[2] {
	    Tag::Region(rd) => {
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
	match &tags[3] {
	    Tag::Region(rd) => {
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
	match &tags[4] {
	    Tag::Region(rd) => {
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
	match &tags[5] {
	    Tag::Region(rd) => {
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
	match &tags[6] {
	    Tag::Region(rd) => {
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
	match &tags[7] {
	    Tag::Region(rd) => {
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
	match &tags[8] {
	    Tag::Region(rd) => {
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
	match &tags[9] {
	    Tag::Region(rd) => {
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
	match &tags[10] {
	    Tag::Region(rd) => {
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
	match &tags[11] {
	    Tag::Region(rd) => {
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
}
