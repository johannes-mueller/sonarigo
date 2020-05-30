
extern crate lv2;

use lv2::prelude::*;

use soundfonts::engine::EngineTrait;
use soundfonts::sfz::engine;

#[derive(PortCollection)]
pub struct Ports {
    control: InputPort<AtomPort>,
    out_left: OutputPort<Audio>,
    out_right: OutputPort<Audio>,
    gain: InputPort<Control>,
}

#[derive(FeatureCollection)]
pub struct Features<'a> {
    map: LV2Map<'a>,
}

#[derive(URIDCollection)]
pub struct URIDs {
    atom: AtomURIDCollection,
    midi: MidiURIDCollection,
    unit: UnitURIDCollection,
}

#[uri("http://johannes-mueller.org/oss/lv2/sonarigo#lv2")]
pub struct SonarigoLV2 {
    engine: engine::Engine,
    urids: URIDs,
}

impl Plugin for SonarigoLV2 {
    type Ports = Ports;

    type InitFeatures = Features<'static>;
    type AudioFeatures = ();

    fn new(plugin_info: &PluginInfo, features: &mut Features<'static>) -> Option<Self> {
	let filename = "/data/Musik/SoundFonts/SalamanderGrandPianoV3_48khz24bit/SalamanderGrandPianoV3.sfz";
	let samplerate = plugin_info.sample_rate();
	let engine = engine::Engine::new(filename.to_string(), samplerate, 8192 /*FIXME*/).ok()?;
	Some(Self {
	    engine: engine,
	    urids: features.map.populate_collection()?
	})
    }

    fn run(&mut self, ports: &mut Ports, _: &mut ()) {
        let mut offset: usize = 0;

        let control_sequence = ports
            .control
            .read(self.urids.atom.sequence, self.urids.unit.beat)
            .unwrap();

	for (timestamp, message) in control_sequence {
            let timestamp: usize = match timestamp.as_frames() {
                Some(ts) => ts as usize,
                None => continue
            };

	    let message = match message.read(self.urids.midi.wmidi, ()) {
		Some(msg) => msg,
		None => continue
	    };

	    self.engine.midi_event(&message);
	}

	self.engine.process(&mut ports.out_left, &mut ports.out_right);

	let gain = match *ports.gain {
	    g if g < -80.0 => 0.0,
	    g => soundfonts::utils::dB_to_gain(g)
	};

	for (l, r) in Iterator::zip(ports.out_left.iter_mut(), ports.out_right.iter_mut()) {
	    *l *= gain;
	    *r *= gain;
	}
    }
}

lv2_descriptors!(SonarigoLV2);
