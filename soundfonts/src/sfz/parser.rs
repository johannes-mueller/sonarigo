
use std::error;
use std::fmt;
use std::num::{ParseIntError, ParseFloatError};

use std::str::Chars;

use super::engine;
use crate::errors::*;

#[derive(Debug)]
pub(super) enum ParserError {
    RangeError(RangeError),
    KeyError(String),
    ParseIntError(ParseIntError),
    ParseFloatError(ParseFloatError),
    NoteParseError(NoteParseError),
    General(String)
}

impl fmt::Display for ParserError{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	match &*self {
	    ParserError::RangeError(re) => re.fmt(f),
	    ParserError::KeyError(k) => write!(f, "Unknown key: {}", k),
	    ParserError::ParseIntError(e) => e.fmt(f),
	    ParserError::ParseFloatError(e) => e.fmt(f),
	    ParserError::NoteParseError(e) => e.fmt(f),
	    ParserError::General(s) => write!(f, "General parser error: {}", s),
	}
    }
}

impl error::Error for ParserError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
	match *self {
	    ParserError::RangeError(ref e) =>  Some(e),
	    ParserError::KeyError(_) => None,
	    ParserError::ParseIntError(ref e) =>  Some(e),
	    ParserError::ParseFloatError(ref e) =>  Some(e),
	    ParserError::NoteParseError(ref e) => Some(e),
	    ParserError::General(_) => None
	}
    }
}


#[derive(Debug)]
pub(super) struct NoteParseError {
    key: String
}

impl fmt::Display for NoteParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	write!(f, "Invalid key: {}", self.key)
    }
}

impl error::Error for NoteParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { None }
}

impl NoteParseError {
    fn new(key: &str) -> NoteParseError {
	NoteParseError { key: key.to_string() }
    }
}


fn parse_key(key: &str) -> Result<i32, NoteParseError> {
    match key.parse::<i32>() {
	Ok(v) => Ok(v),
	Err(_) => {
	    let mut bytes = key.bytes();
	    if bytes.len() < 2 {
		return Err(NoteParseError::new(key))
	    }
	    let name = match bytes.next().unwrap() {
		k if k >= 'a' as u8 => k - 0x20, //uppercase
		k => k
	    };
	    let note_val = match name as char {
		'C' => 0,
		'D' => 2,
		'E' => 4,
		'F' => 5,
		'G' => 7,
		'A' => 9,
		'B' => 11,
		_ => return Err(NoteParseError::new(key))
	    };
	    let second_byte = bytes.next().unwrap();
	    let sign = match second_byte as char {
		'#' => 1,
		'b' => -1,
		_ => 0
	    };

	    let octave_char = match sign {
		0 => second_byte,
		_ => match bytes.next() {
		    None => return Err(NoteParseError::new(key)),
		    Some(v) => v
		}
	    };
	    let octave = (octave_char - '0' as u8) as i8;
	    if octave < 0 || octave > 9 {
		return Err(NoteParseError::new(key))
	    }
	    Ok(((octave + 1) * 12 + (note_val + sign)) as i32)
	}
    }
}

#[derive(Debug)]
enum NextChar {
    None,
    NewTag,
    Some(char)
}

fn next_char(chars: &mut Chars) -> NextChar {
    match chars.next() {
	None => NextChar::None,
	Some('/') => {
	    while let Some(c) = chars.next() {
		if c == '\n' {
		    break;
		}
	    }
	    next_char(chars)
	}
	Some('<') => NextChar::NewTag,
	Some(c) =>  NextChar::Some(c)
    }
}

fn next_char_skip_whitespace(chars: &mut Chars) -> NextChar {
    let nc = next_char(chars);
    match nc {
	NextChar::Some(' ')  |
	NextChar::Some('\t') |
	NextChar::Some('\n') |
	NextChar::Some('\r') => next_char_skip_whitespace(chars),
	_ => nc
    }
}

fn parse_header(chars: &mut Chars) -> Result<String, ParserError> {
    let mut header_string = String::new();
    loop {
	match next_char(chars) {
	    NextChar::Some('>') => break Ok(header_string),
	    NextChar::None => break Err(ParserError::General("File ended before opcode name finished".to_string())),
	    NextChar::Some(c) => { header_string.push(c); }
	    NextChar::NewTag => break Err(ParserError::General("Tag begin (<) while parsing tag header".to_string()))
	}
    }
}

fn parse_opcode(chars: &mut Chars) -> Result<(Option<(String, String)>, NextChar), ParserError> {
    let mut opcode_string = String::new();

    let mut nc = next_char_skip_whitespace(chars);
    while let NextChar::Some(c) = nc {
	match c {
	    '=' => break,
	    _ => opcode_string.push(c)
	}
	nc = next_char(chars);
    }

    match nc {
	NextChar::NewTag => {
	    if !opcode_string.is_empty() {
		return Err(ParserError::General("New tag starts while scanning obcode key".to_string()));
	    } else {
		return Ok((None, NextChar::NewTag))
	    }
	}
	NextChar::None => return Ok((None, NextChar::None)),
	_ => {}
    }

    let mut value_string = String::new();
    nc = next_char_skip_whitespace(chars);

    while let NextChar::Some(c) = nc {
	match c {
	    ' ' | '\t' | '\n' | '\r' => break,
	    _ => { value_string.push(c); }
	}
	nc = next_char(chars);
    };

    Ok((Some((opcode_string.trim().to_string(), value_string.trim().to_string())), nc))
}



fn take_opcode(region: &mut engine::RegionData, key: &str, value: &str) -> Result<(), ParserError> {
    match key {
	"lokey" => region.key_range.set_lo(parse_key(value).map_err(|ne| ParserError::NoteParseError(ne))?).map_err(|re| ParserError::RangeError(re)),
	"hikey" => region.key_range.set_hi(parse_key(value).map_err(|ne| ParserError::NoteParseError(ne))?).map_err(|re| ParserError::RangeError(re)),
	"pitch_keycenter" => region.set_pitch_keycenter(value.parse::<u32>().map_err(|pe| ParserError::ParseIntError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"key" => {
	    let key = parse_key(value).map_err(|ne| ParserError::NoteParseError(ne))?;
	    match key {
		k if k < 0 => Err(RangeError::out_of_range("key", 0, 127, key)),
		k => region.key_range.set_hi(k).and_then(|_| region.key_range.set_lo(k)).and_then(|_| region.set_pitch_keycenter((k as u8).into()))
	    }
	}.map_err(|re| ParserError::RangeError(re)),
	"lovel" => region.vel_range.set_lo(value.parse::<i32>().map_err(|pe| ParserError::ParseIntError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"hivel" => region.vel_range.set_hi(value.parse::<i32>().map_err(|pe| ParserError::ParseIntError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"lorand" => region.random_range.set_lo(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"hirand" => region.random_range.set_hi(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"tune" => region.set_tune(value.parse::<i32>().map_err(|pe| ParserError::ParseIntError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"volume" => region.set_volume(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"rt_decay" => region.set_rt_decay(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"pitch_keytrack" => region.set_pitch_keytrack(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"amp_veltrack" => region.set_amp_veltrack(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"ampeg_attack" => region.ampeg.set_attack(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"ampeg_hold" => region.ampeg.set_hold(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"ampeg_decay" => region.ampeg.set_decay(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"ampeg_sustain" => region.ampeg.set_sustain(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"ampeg_release" => region.ampeg.set_release(value.parse::<f32>().map_err(|pe| ParserError::ParseFloatError(pe))?).map_err(|re| ParserError::RangeError(re)),
	"group" => { region.set_group(value.parse::<u32>().map_err(|pe| ParserError::ParseIntError(pe))?); Ok(()) },
	"off_by" => { region.set_off_by(value.parse::<u32>().map_err(|pe| ParserError::ParseIntError(pe))?); Ok(()) },
	"sample" => { region.set_sample(value); Ok(()) },
	"trigger" => { region.set_trigger(parse_trigger(value)?); Ok(()) },
	s => {
	    match s.find("cc") {
		None => {},
		Some(n) => {
		    let (key_cc, ns) = s.split_at(n);
		    let cc_num = ns.get(2..).unwrap().parse::<u32>().map_err(|pe| ParserError::ParseIntError(pe))?;
		    if cc_num > 127 {
			return Err(ParserError::RangeError(RangeError::out_of_range("cc number", 0, 127, cc_num)))
		    }
		    let value = value.parse::<i32>().map_err(|pe| ParserError::ParseIntError(pe))?;

		    match key_cc {
			"on_lo" => region.set_on_lo_cc(cc_num, value),
			"on_hi" => region.set_on_hi_cc(cc_num, value),
			_ => return Err(ParserError::KeyError(key_cc.to_string()))
		    }
		    return Ok(());
		}
	    };
	    return Err(ParserError::KeyError(key.to_string()))
	}
    }
}

fn parse_trigger(s: &str) -> Result<engine::Trigger, ParserError> {
	 match s {
	    "attack" => Ok(engine::Trigger::Attack),
	    "release" => Ok(engine::Trigger::Release),
	    "first" => Ok(engine::Trigger::First),
	    "legato" => Ok(engine::Trigger::Legato),
	    "release_key" => Ok(engine::Trigger::ReleaseKey),
	    _ => Err(ParserError::KeyError(s.to_string()))
	}
}


fn parse_region(chars: &mut Chars, mut region: engine::RegionData) -> Result<(engine::RegionData, NextChar), ParserError> {

    let nc = loop {
	match parse_opcode(chars) {
	    Err(e) => return Err(e),
	    Ok((nop, nc)) => {
		match nop {
		    Some((opcode, value)) => {
			take_opcode(&mut region, opcode.trim(), value.trim())?
		    }
		    None => break nc
		}
		match nc {
		    NextChar::NewTag => break NextChar::NewTag,
		    _ => {}
		}
	    }
	}
    };

    Ok((region, nc))
}

pub(super) fn parse_sfz_text(text: String) -> Result<Vec<engine::RegionData>, ParserError> {
    let mut chars = text.chars();

    let mut current_group = engine::RegionData::default();

    let mut regions = vec![];

    match next_char_skip_whitespace(&mut chars) {
	NextChar::NewTag => {},
	NextChar::None | NextChar::Some(_) => return Err(ParserError::General("Expecting <> tag in sfz file".to_string()))
    };

    loop {
	let header_string = parse_header(&mut chars)?;

	let nc = match header_string.trim() {
	    "group" => {
		let (grp, nc) = parse_region(&mut chars, engine::RegionData::default())?;
		current_group = grp;
		nc
	    }
	    "region" => {
		let (reg, nc) = parse_region(&mut chars, current_group.clone())?;
		regions.push(reg);
		nc
	    }
	    s => return Err(ParserError::KeyError(s.to_string()))
	};

	match nc {
	    NextChar::NewTag => {}
	    _ => break
	}
    }

    Ok(regions)
}
