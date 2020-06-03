use std::error;
use std::fmt;

#[derive(Debug)]
pub struct FlippedRangeError {
    key: &'static str,
    wrong: String,
    other: String,
}

impl fmt::Display for FlippedRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Flipped range {}: {} <-> {}", self.key, self.wrong, self.other)
    }
}

#[derive(Debug)]
pub struct OutOfRangeError {
    key: &'static str,
    hi: String,
    lo: String,
    actual: String,
}

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} out of range: {} <= {} <= {}", self.key, self.lo, self.actual, self.hi)
    }
}

#[derive(Debug)]
pub enum RangeError {
    FlippedRange(FlippedRangeError),
    OutOfRange(OutOfRangeError),
}

impl RangeError {
    pub fn out_of_range<T: ToString>(key: &'static str, lo: T, hi: T, actual: T) -> RangeError {
        RangeError::OutOfRange(OutOfRangeError {
            key,
            hi: hi.to_string(),
            lo: lo.to_string(),
            actual: actual.to_string(),
        })
    }
    pub fn flipped_range<T: ToString>(key: &'static str, wrong: T, other: T) -> RangeError {
        RangeError::FlippedRange(FlippedRangeError {
            key,
            wrong: wrong.to_string(),
            other: other.to_string(),
        })
    }
}

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            RangeError::FlippedRange(fr) => fr.fmt(f),
            RangeError::OutOfRange(or) => or.fmt(f),
        }
    }
}

impl error::Error for RangeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub fn range_check<T>(v: T, lo: T, hi: T, name: &'static str) -> Result<T, RangeError>
where T: PartialOrd + fmt::Display {
    match v {
        v if v >= lo && v <= hi => {
            Ok(v)
        }
        _ => Err(RangeError::out_of_range(name, lo, hi, v))
    }
}
