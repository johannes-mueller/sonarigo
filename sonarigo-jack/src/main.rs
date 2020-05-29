
use std::env;
use std::io::Write;
use std::convert::TryFrom;
use std::io;

extern crate jack;
extern crate wmidi;

use soundfonts::engine::EngineTrait;
use soundfonts::sfz::engine;

fn main() {
    let (client, _status) = match jack::Client::new("Sonarigo", jack::ClientOptions::NO_START_SERVER) {
	Err(e) => {
	    println!("Failed to connecect to jack server: {:?}:", e);
	    return
	}
	Ok(cs) => cs
    };

    let samplerate = client.sample_rate();
    let max_block_length = client.buffer_size();
    println!("Samplerate: {}; maximum buffer size: {}", samplerate, max_block_length);

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    let mut engine = match engine::Engine::new(filename.to_string(), samplerate as f64, max_block_length as usize) {
	Err(e) => {
	    println!("Could not launch SFZ engine: {:?}", e);
	    return
	}
	Ok(e) => e
    };

    let midi_in = match client.register_port("MIDI input", jack::MidiIn::default()) {
	Err(e) => {
	    println!("MIDI input port registration failed: {:?}:", e);
	    return
	}
	Ok(p) => p
    };

    let mut out_left = match client.register_port("out left", jack::AudioOut::default()) {
	Err(e) => {
	    println!("Audio output port registration failed: {:?}:", e);
	    return
	}
	Ok(p) => p
    };

    let mut out_right = match client.register_port("out right", jack::AudioOut::default()) {
	Err(e) => {
	    println!("Audio output port registration failed: {:?}:", e);
	    return
	}
	Ok(p) => p
    };

    let callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
	for e in midi_in.iter(ps) {
	    let midi_msg = match wmidi::MidiMessage::try_from(e.bytes) {
		Ok(m) => m,
		Err(e) => {
		    println!("midi event conversion failed: {:?}", e);
		    continue
		}
	    };
	    println!("{:?}", midi_msg);
	    engine.midi_event(&midi_msg);
	    io::stdout().flush();
	}

	let left = out_left.as_mut_slice(ps);
	let right = out_right.as_mut_slice(ps);
	engine.process(left, right);

	jack::Control::Continue
    };

    let active_client = match client.activate_async((), jack::ClosureProcessHandler::new(callback)) {
	Err(e) => {
	    println!("Could not activate client: {:?}", e);
	    return
	}
	Ok(a) => a,
    };

    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    active_client.deactivate().unwrap();
}
