
extern crate lv2;

use lv2::prelude::*;
use lv2::lv2_atom as atom;

use soundfonts::engine::EngineTrait;
use soundfonts::sfz::engine;

mod patch;

#[uri("http://lv2plug.in/ns/ext/atom#Path")]
struct AtomPath;

impl<'a, 'b> Atom<'a, 'b> for AtomPath
where
    'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = &'a str;

    type WriteParameter = ();
    type WriteHandle = AtomPathWriter<'a, 'b>;

    fn read(body: Space<'a>, _: ()) -> Option<&'a str> {
	body.data()
            .and_then(|data| std::str::from_utf8(data).ok())
            .map(|string| &string[..string.len() - 1])
    }

    fn init(frame: FramedMutSpace<'a, 'b>, _: ()) -> Option<AtomPathWriter<'a, 'b>> {
        Some(AtomPathWriter { frame })
    }
}

struct AtomPathWriter<'a, 'b> {
    frame: FramedMutSpace<'a, 'b>
}


#[uri("http://lv2plug.in/ns/ext/state#StateChanged")]
struct StateChanged;

#[uri("http://johannes-mueller.org/oss/lv2/sonarigo#sfzfile")]
struct SampleFile;


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
    patch: patch::PatchURIDCollection,
    state_changed: URID<StateChanged>,
    atom_path: URID<AtomPath>,

    sfzfile: URID<SampleFile>,
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
            match timestamp.as_frames() {
		Some(ts) if ts > 0  => {
		    let frame = ts as usize;
		    self.engine.process(&mut ports.out_left[offset..frame], &mut ports.out_right[offset..frame]);
		    offset = frame;
		}
		_ => {}
            };

	    if let Some(msg) = message.read(self.urids.midi.wmidi, ()) {
		self.engine.midi_event(&msg);
	    };

	    if let Some((header, mut object_reader)) = message.read(self.urids.atom.object, ()) {
		if header.otype != self.urids.patch.set {
		    continue;
		}

		if let Some(path) = self.parse_sfzfile_path(&mut object_reader) {
		    println!("received path {}", path);
		}
	    }
	}

	let nsamples = ports.out_left.len();
	if offset < nsamples {
	    self.engine.process(&mut ports.out_left[offset..nsamples], &mut ports.out_right[offset..nsamples]);
	}

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

impl SonarigoLV2 {
    fn parse_sfzfile_path<'a>(&self, object_reader: &mut atom::object::ObjectReader<'a>) -> Option<&'a str> {
	if let Some((property_header, atom)) = object_reader.next() {
	    if property_header.key != self.urids.patch.property {
		return None;
	    }
	    if atom.read(self.urids.atom.urid, ()).map_or(true, |urid| urid != self.urids.sfzfile) {
		return None;
	    }
	    if let Some((property_header, atom)) = object_reader.next() {
		if property_header.key != self.urids.patch.value {
		    return None;
		}
		let path = if let Some(path) = atom.read(self.urids.atom_path, ()) {
		    path
		} else {
		    return None;
		};
		return Some(path);
	    }
	}
	None
    }
}

lv2_descriptors!(SonarigoLV2);
