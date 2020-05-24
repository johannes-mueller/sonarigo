
use std::f32;

#[allow(non_snake_case)]
pub(crate) fn dB_to_gain(dB: f32) -> f32 {
    let ten: f32 = 10.0;
    ten.powf(0.05 * dB)
}
