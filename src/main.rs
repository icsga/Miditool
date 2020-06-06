//! MIDI Toolbox: A few little helpers for terminal MIDI handling
//!
//! Working:
//! * Forward MIDI data from one port to another
//! * Transform MIDI data (e.g. change the channel)
//! * Monitor the received data
//!
//! TODO:
//! * Improve the logging
//! * Log the received data to a file
//! * Replay a previously captured file
//! * Send a MIDI file to a device

extern crate clap;
use clap::{Arg, App};

extern crate midir;
use midir::{MidiInput, MidiOutput, MidiIO, Ignore};

use std::io::stdin;
use std::error::Error;

fn main() {
    let matches = App::new("MIDIToolbox")
                        .version("0.1.0")
                        .about("Some MIDI utilities for the terminal")
                        .arg(Arg::with_name("version")
                            .short("v")
                            .long("version")
                            .help("Shows the version of the app"))
                        .arg(Arg::with_name("inport")
                            .short("i")
                            .long("inport")
                            .help("Selects the MIDI port to receive MIDI events on (0 - n, default 0)")
                            .takes_value(true))
                        .arg(Arg::with_name("outport")
                            .short("o")
                            .long("outport")
                            .help("Selects the MIDI port to send MIDI events to (0 - n, default 1)")
                            .takes_value(true))
                        .arg(Arg::with_name("inchannel")
                            .short("c")
                            .long("inchannel")
                            .help("Selects the MIDI channel to receive MIDI events on (1 - 16, default = omni)")
                            .takes_value(true))
                        .arg(Arg::with_name("outchannel")
                            .short("n")
                            .long("outchannel")
                            .help("Selects the MIDI channel to send MIDI events on (1 - 16, default = omni)")
                            .takes_value(true))
                        .arg(Arg::with_name("monitor")
                            .short("m")
                            .long("monitor")
                            .help("Activates monitoring of MIDI events"))
                        .arg(Arg::with_name("list")
                            .short("l")
                            .long("list")
                            .help("List available MIDI ports and exit"))
                        .get_matches();
    let in_port = matches.value_of("inport").unwrap_or("0");
    let in_port: usize = in_port.parse().unwrap_or(0);
    let in_channel = matches.value_of("inchannel").unwrap_or("0");
    let in_channel: u8 = in_channel.parse().unwrap_or(0);
    let out_port = matches.value_of("outport").unwrap_or("1");
    let out_port: usize = out_port.parse().unwrap_or(1);
    let out_channel = matches.value_of("outchannel").unwrap_or("0");
    let out_channel: u8 = out_channel.parse().unwrap_or(0);
    let monitor: bool = matches.is_present("monitor");
    let list: bool = matches.is_present("list");

    if list {
        match list_all_ports() {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err)
        }
        return;
    }

    match run(in_port, in_channel, out_port, out_channel, monitor) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err)
    }
}

fn run(in_port: usize, in_channel: u8, out_port: usize, out_channel: u8, monitor: bool)
        -> Result<(), Box<dyn Error>> {

    let mut midi_in = MidiInput::new("MIDI input")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("MIDI output")?;

    let in_port = get_port(&midi_in, in_port)?;
    let out_port = get_port(&midi_out, out_port)?;

    println!("\nOpening connections");
    let in_port_name = midi_in.port_name(&in_port)?;
    let out_port_name = midi_out.port_name(&out_port)?;

    let mut conn_out = midi_out.connect(&out_port, "midir-forward")?;
    let mut message_out: [u8; 3] = [0x00, 0x00, 0x00];

    let _conn_in = midi_in.connect(&in_port, "midir-forward", move |stamp, message, _| {

        if message.len() >= 2 {
            message_out[0] = message[0];
            message_out[1] = message[1];
            if message.len() == 3 {
                message_out[2] = message[2];
            }

            if in_channel < 16 {
                // Do checks and modifications of received message
                if (message[0] & 0x0F) != in_channel {
                    return; // Received message on wrong channel
                }
                if out_channel < 16 && out_channel != in_channel {
                    message_out[0] = message[0] & 0xF0 | out_channel;
                }
            }
        } else {
            println!("Got MIDI message with len {}", message.len());
        }

        conn_out.send(&message_out).unwrap_or_else(|_| println!("Error when forwarding message ..."));
        if monitor {
            println!("{}: {:?} (len = {})", stamp, message, message.len());
        }
    }, ())?;

    println!("Forwarding from '{}' to '{}', press enter to exit.", in_port_name, out_port_name);

    let mut input = String::new();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connections");
    Ok(())
}

fn get_port<T: MidiIO>(midi_io: &T, port: usize) -> Result<T::Port, Box<dyn Error>> {
    let midi_ports = midi_io.ports();
    let port = midi_ports.get(port)
                         .ok_or("Invalid port number")?;
    Ok(port.clone())
}

fn list_all_ports()
        -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("MIDI input")?;
    midi_in.ignore(Ignore::None);
    list_ports(&midi_in, "input")?;

    let midi_out = MidiOutput::new("MIDI output")?;
    list_ports(&midi_out, "output")
}

fn list_ports<T: MidiIO>(midi_io: &T, descr: &str)
        -> Result<(), Box<dyn Error>> {
    println!("\nAvailable {} ports:", descr);
    let midi_ports = midi_io.ports();
    for (i, p) in midi_ports.iter().enumerate() {
        println!("{}: {}", i, midi_io.port_name(p)?);
    }
    Ok(())
}

