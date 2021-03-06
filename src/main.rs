//! MIDI Toolbox: A little helper for terminal MIDI handling
//!
//! Features:
//! * Forward MIDI data between different ports
//! * Transform MIDI data (e.g. change the channel)
//! * Monitor the received data
//! * Log the received data to a file
//!
//! TODO:
//! * Replay a previously captured file
//! * Send a MIDI file to a device

mod avg;
use avg::Avg;

mod display;
use display::{Display, Colors, COLORS_BW, COLORS_TC};

mod midi;
use midi::MidiMessage;

extern crate clap;
use clap::{Arg, App};

extern crate midir;
use midir::{MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection, MidiIO, Ignore};

extern crate regex;
use regex::Regex;

use std::error::Error;
use std::fs::File;
use std::io::stdin;
use std::io::prelude::*;
use std::io::BufReader;

struct Config {
    in_port: usize,
    in_channel: u8,
    out_port: usize,
    out_channel: u8,
}

fn main() {
    let mut config = Config{
        in_port: std::usize::MAX,
        in_channel: 0,
        out_port: std::usize::MAX,
        out_channel: 0,
    };

    let matches = App::new("MIDIToolbox")
                        .version("0.2.0")
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
                            .help("Selects the MIDI port to send MIDI events to (0 - n, default OFF)")
                            .takes_value(true))
                        .arg(Arg::with_name("inchannel")
                            .short("c")
                            .long("inchannel")
                            .help("Selects the MIDI channel to receive MIDI events on (1 - 16, 0 = omni (default))")
                            .takes_value(true))
                        .arg(Arg::with_name("outchannel")
                            .short("n")
                            .long("outchannel")
                            .help("Selects the MIDI channel to send MIDI events on (1 - 16, 0 = omni (default))")
                            .takes_value(true))
                        .arg(Arg::with_name("monitor")
                            .short("m")
                            .long("monitor")
                            .help("Print received MIDI events to stdout"))
                        .arg(Arg::with_name("write")
                            .short("w")
                            .long("write")
                            .help("Record the received MIDI events to a file")
                            .takes_value(true))
                        .arg(Arg::with_name("list")
                            .short("l")
                            .long("list")
                            .help("List available MIDI ports and exit"))
                        .arg(Arg::with_name("configfile")
                            .short("r")
                            .long("read")
                            .help("Read a CSV file containing a multiplex/ demultiplex setup. Each line consists of a single entry of the form \"inport, inchannel, outport, outchannel\"")
                            .takes_value(true))
                        .arg(Arg::with_name("blackwhite")
                            .short("b")
                            .long("no-color")
                            .help("Don't use color when printing events."))
                        .arg(Arg::with_name("timing")
                            .short("t")
                            .long("show-timing")
                            .help("Show system real-time messages."))
                        .get_matches();
    let in_port = matches.value_of("inport").unwrap_or("");
    config.in_port = in_port.parse().unwrap_or(std::usize::MAX);
    let in_channel = matches.value_of("inchannel").unwrap_or("0");
    config.in_channel = in_channel.parse().unwrap_or(0);
    let out_port = matches.value_of("outport").unwrap_or("");
    config.out_port = out_port.parse().unwrap_or(std::usize::MAX);
    let out_channel = matches.value_of("outchannel").unwrap_or("0");
    config.out_channel = out_channel.parse().unwrap_or(0);
    let monitor = matches.is_present("monitor");
    let list = matches.is_present("list");
    let record = matches.is_present("write");
    let outfile = matches.value_of("write").unwrap_or("");
    let show_time = matches.is_present("timing");

    if list {
        match list_all_ports() {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err)
        }
        return;
    }

    // Set colors to use for output
    let colors = if matches.is_present("blackwhite") {
        &COLORS_BW
    } else {
        &COLORS_TC
    };

    let mut configs: Vec<Config> = vec!();
    if matches.is_present("configfile") {
        let re = Regex::new(r"(\d*),(\d*),(\d*),(\d)").unwrap();
        let configfile = matches.value_of("configfile").unwrap_or("");
        let file = File::open(configfile).unwrap(); // TODO: Show error
        let buf_reader = BufReader::new(file);
        let lines = buf_reader.lines();
        for line in lines {
            let line = if let Ok(l) = line { l } else { continue; };
            let cap = re.captures(&line).unwrap();
            if cap.len() == 5 {
                let c = Config{
                    in_port: cap[1].parse().unwrap(),
                    in_channel: cap[2].parse().unwrap(),
                    out_port: cap[3].parse().unwrap(),
                    out_channel: cap[4].parse().unwrap(),
                };
                configs.push(c);
            }
        }
    } else {
        configs.push(config);
    }

    match receive_data(&configs, monitor, record, outfile, colors, show_time) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err)
    }
}

/// Receive data from a MIDI in port and optionally forward it.
///
/// If no output port has been defined, the data is only read, written to file
/// if configured, and written to stdout if configured.
fn receive_data(configs: &[Config],
                do_monitor: bool,
                do_record: bool,
                outfile: &str,
                colors: &'static Colors,
                show_time: bool)
        -> Result<(), Box<dyn Error>> {

    let mut conn_list = vec!();

    for config in configs {
        let mut display = Display::new(colors, show_time);
        let mut midi_in = MidiInput::new("MIDI input")?;
        midi_in.ignore(Ignore::None);
        let conf_in_port = config.in_port;
        let in_port = get_in_port(config, &midi_in)?;
        let in_channel = config.in_channel;

        let do_forward = config.out_port < std::usize::MAX;
        let mut conn_out = get_out_connection(config)?;
        let mut message_out: [u8; 3] = [0x00, 0x00, 0x00];
        let out_channel = config.out_channel;

        let mut file = if do_record {
            let mut filename = outfile.to_string();
            filename += "_p";
            filename += &config.in_port.to_string();
            Some(File::create(filename)?)
        } else {
            None
        };

        let conn_in = midi_in.connect(&in_port, "MIDI forward", move |timestamp, message, _| {

            if in_channel > 0 && (message[0] & 0x0F) != in_channel - 1 {
                return; // Not listening on this channel
            }

            if do_forward {
                // Filter some messages (for Push2)
                let m = MidiMessage::parse(message);
                match m {
                    MidiMessage::NoteOn{channel: _, key, velocity: _} => {
                        if key <= 10 {
                            return;
                        }
                    }
                    _ => (),
                }

                // Forward data to configured output port
                if out_channel < 16 && out_channel != in_channel {
                    // Adjust MIDI channel
                    message_out[0] = message[0] & 0xF0 | out_channel - 1;
                } else {
                    message_out[0] = message[0];
                }
                if message.len() > 1 {
                    message_out[1] = message[1];
                    if message.len() == 3 {
                        message_out[2] = message[2];
                    }
                }
                if let Some(c) = conn_out.as_mut() {
                    c.send(&message_out).unwrap_or_else(|_| println!("Error when forwarding message ..."));
                }
            }

            if do_monitor {
                // Print received data to screen
                display.show_message(timestamp, conf_in_port, message);
            }

            if do_record {
                // Write received data to file
                if let Some(f) = file.as_mut() {
                    let line = if message.len() == 3 {
                        format!("{:02x} {:02x} {:02x}\n", message[0], message[1], message[2])
                    } else if message.len() == 2 {
                        format!("{:02x} {:02x}\n", message[0], message[1])
                    } else {
                        "\n".to_string()
                    };
                    f.write_all(line.as_bytes()).unwrap();
                }
            }
        }, ())?;
        conn_list.push(conn_in);
    }

    println!("Press return to exit.");
    let mut input = String::new();
    stdin().read_line(&mut input)?;

    Ok(())
}

fn get_in_port(config: &Config, midi_in: &MidiInput) -> Result<MidiInputPort, Box<dyn Error>> {
    let conf_in_port = config.in_port;
    let in_port = get_port(midi_in, conf_in_port)?;
    let in_port_name = midi_in.port_name(&in_port)?;
    print!("Reading from '{}'", in_port_name);
    if config.in_channel > 0 {
        print!(", channel {}", config.in_channel);
    } else {
        print!(", all channels");
    }
    Ok(in_port)
}

fn get_out_connection(config: &Config) -> Result<Option<MidiOutputConnection>, Box<dyn Error>> {
    let do_forward = config.out_port < std::usize::MAX;
    let conn_out: Option<MidiOutputConnection> = if do_forward {
        let midi_out = MidiOutput::new("MIDI output")?;
        let out_port = get_port(&midi_out, config.out_port)?;
        let out_port_name = midi_out.port_name(&out_port)?;
        print!(", forwarding to '{}'", out_port_name);
        if config.out_channel > 0 {
            println!(", channel {}", config.out_channel);
        } else {
            println!(", all channels");
        }
        Some(midi_out.connect(&out_port, "MIDI forward")?)
    } else {
        println!("");
        None
    };
    Ok(conn_out)
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

