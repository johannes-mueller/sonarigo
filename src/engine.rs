
use wmidi;

pub trait EngineTrait {
    fn midi_event(&mut self, midi_msg: wmidi::MidiMessage);

    fn process(&mut self, out_left: &mut [f32], out_right: &mut [f32]);
}
