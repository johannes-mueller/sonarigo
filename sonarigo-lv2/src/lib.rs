use std::any::Any;

use std::f32::consts::PI;

extern crate lv2;
extern crate lv2_worker;

use lv2::prelude::*;
use lv2::lv2_atom as atom;

use soundfonts::engine::EngineTrait;
use soundfonts::sfz::engine;

mod lv2_stuff;

#[uri("http://lv2plug.in/ns/ext/state#StateChanged")]
struct StateChanged;

#[uri("http://johannes-mueller.org/oss/lv2/sonarigo#sfzfile")]
struct SampleFile;


#[derive(PortCollection)]
struct Ports {
    control: InputPort<AtomPort>,
    notify: OutputPort<AtomPort>,
    out_left: OutputPort<Audio>,
    out_right: OutputPort<Audio>,
    gain: InputPort<Control>,
}

#[derive(FeatureCollection)]
struct Features<'a> {
    map: LV2Map<'a>,
}

#[derive(FeatureCollection)]
struct AudioFeatures<'a> {
    schedule: lv2_worker::Schedule<'a, SonarigoLV2>,
}

#[derive(URIDCollection)]
struct URIDs {
    atom: AtomURIDCollection,
    midi: MidiURIDCollection,
    unit: UnitURIDCollection,
    patch: lv2_stuff::PatchURIDCollection,
    state_changed: URID<StateChanged>,
    atom_path: URID<lv2_stuff::AtomPath>,

    sfzfile: URID<SampleFile>,
}


#[uri("http://johannes-mueller.org/oss/lv2/sonarigo#lv2")]
struct SonarigoLV2 {
    engine: engine::Engine,
    new_engine: Option<engine::Engine>,
    urids: URIDs,

    sfzfile_path: Option<std::string::String>,

    samplerate: f64,
    max_block_length: usize,

    state_notification_needed: bool,

    current_gain: f32
}

impl Plugin for SonarigoLV2 {
    type Ports = Ports;

    type InitFeatures = Features<'static>;
    type AudioFeatures = AudioFeatures<'static>;

    fn new(plugin_info: &PluginInfo, features: &mut Features<'static>) -> Option<Self> {
        let samplerate = plugin_info.sample_rate();
        let max_block_length = 8192; /*FIXME*/
        let engine = engine::Engine::dummy(samplerate, max_block_length);
        Some(Self {
            engine,
            new_engine: None,
            urids: features.map.populate_collection()?,

            sfzfile_path: None,

            samplerate,
            max_block_length,

            state_notification_needed: false,

            current_gain: soundfonts::utils::dB_to_gain(-6.0)
        })
    }

    fn run(&mut self, ports: &mut Ports, features: &mut Self::AudioFeatures, _: u32) {
        let mut offset: usize = 0;

        for (l, r) in Iterator::zip(ports.out_left.iter_mut(), ports.out_right.iter_mut()) {
            *l = 0.0;
            *r = 0.0;
        }

        let active_engine = if let Some(new_engine) = &mut self.new_engine {
            if self.engine.fadeout_finished() {
                self.engine = self.new_engine.take().unwrap();
                &mut self.engine
            } else {
                self.engine.process(&mut ports.out_left, &mut ports.out_right);
                new_engine
            }
        } else {
            &mut self.engine
        };

        let control_sequence = ports
            .control
            .read(self.urids.atom.sequence, self.urids.unit.beat)
            .unwrap();

        for (timestamp, message) in control_sequence {
            match timestamp.as_frames() {
                Some(ts) if ts > 0  => {
                    let frame = ts as usize;
                    active_engine.process(&mut ports.out_left[offset..frame],
                                          &mut ports.out_right[offset..frame]);
                    offset = frame;
                }
                _ => {}
            };

            if let Some(msg) = message.read(self.urids.midi.wmidi, ()) {
                active_engine.midi_event(&msg);
            };

            if let Some((header, mut object_reader)) = message.read(self.urids.atom.object, ()) {
                println!("received message");
                if header.otype == self.urids.patch.set {
                    if let Some(path) = parse_sfzfile_path(&self.urids, &mut object_reader) {
                        if let Err(e) = features.schedule.schedule_work(EngineParameters {
                            sfzfile: path.to_string(),
                            host_samplerate: self.samplerate,
                            max_block_length: self.max_block_length
                        }) {
                            println!("can't schedule work {}", e);
                        } else {
                            println!("work scheduled");
                        }
                        self.sfzfile_path = Some(path.to_string());
                    }
                } else if header.otype == self.urids.patch.get {
                    println!("recieved get request");
                    self.state_notification_needed = true;
                }
            }
        }

        let nsamples = ports.out_left.len();
        if offset < nsamples {
            active_engine.process(&mut ports.out_left[offset..nsamples],
                                  &mut ports.out_right[offset..nsamples]);
        }

        let gain_target = match *ports.gain {
            g if g < -80.0 => 0.0,
            g if g >= 20.0 => soundfonts::utils::dB_to_gain(20.0),
            g => soundfonts::utils::dB_to_gain(g)
        };

        let tau = 1.0 - (-2.0 * PI * 25.0 / self.samplerate as f32).exp();
        let mut current_gain = self.current_gain;

        for (l, r) in Iterator::zip(ports.out_left.iter_mut(), ports.out_right.iter_mut()) {
            current_gain += tau * (gain_target - current_gain);
            *l *= current_gain;
            *r *= current_gain;
        }

	if (tau * (current_gain - gain_target)).abs() < std::f32::EPSILON * current_gain {
		current_gain = gain_target;
	}
        self.current_gain = current_gain;

        if self.state_notification_needed {//&& self.sfzfile_path.is_some() {
            println!("trying to notify");

            let mut object_writer = ports.notify.init(
                self.urids.atom.object,
                ObjectHeader {
                    id: None,
                    otype: self.urids.patch.set.into_general(),
                }
            ).unwrap();

            object_writer.init(self.urids.patch.property,
                               self.urids.atom.urid,
                               self.urids.sfzfile.into_general());

            let mut prop_writer = object_writer.init(self.urids.patch.value,
                                                 self.urids.atom_path, ()).unwrap();
            let test_string = prop_writer.append(self.sfzfile_path.as_ref().unwrap());

            println!("wrote {:?}", test_string);

            self.state_notification_needed = false;
        }

    }

    fn extension_data(uri: &Uri) -> Option<&'static dyn Any> {
        match_extensions![uri, lv2_worker::WorkerDescriptor<Self>]
    }
}

fn parse_sfzfile_path<'a>(urids: &URIDs, object_reader:
                          &mut atom::object::ObjectReader<'a>) -> Option<&'a str> {
    if let Some((property_header, atom)) = object_reader.next() {
        if property_header.key != urids.patch.property {
            return None;
        }
        if atom.read(urids.atom.urid, ()).map_or(true, |urid| urid != urids.sfzfile) {
            return None;
        }
        if let Some((property_header, atom)) = object_reader.next() {
            if property_header.key != urids.patch.value {
                return None;
            }
            let path = if let Some(path) = atom.read(urids.atom_path, ()) {
                path
            } else {
                return None;
            };
            return Some(path);
        }
    }
    None
}

struct EngineParameters {
    sfzfile: std::string::String,
    host_samplerate: f64,
    max_block_length: usize
}

impl lv2_worker::Worker for SonarigoLV2 {
    type WorkData = EngineParameters;

    type ResponseData = soundfonts::sfz::engine::Engine;

    fn work(response_handler: &lv2_worker::ResponseHandler<Self>, data: Self::WorkData)
            -> Result<(), lv2_worker::WorkerError> {
        println!("work {}", data.sfzfile);
        let engine = soundfonts::sfz::engine::Engine::new(data.sfzfile,
                                                          data.host_samplerate,
                                                          data.max_block_length)
            .map_err(|e| {
                println!("failed {:?}", e);
                lv2_worker::WorkerError::Unknown
            })?;

        response_handler.respond(engine).map_err(|_| lv2_worker::WorkerError::Unknown)
    }

    fn work_response(&mut self, data: Self::ResponseData, _f: &mut Self::AudioFeatures)
                     -> Result<(), lv2_worker::WorkerError> {
        println!("work_response");
        self.engine.fadeout();
        self.new_engine = Some(data);
        self.state_notification_needed = true;

        Ok(())
    }
}
lv2_descriptors!(SonarigoLV2);
