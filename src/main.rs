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

enum MidiMessage {
    NoteOff    {channel: u8, key: u8, velocity: u8},
    NoteOn     {channel: u8, key: u8, velocity: u8},
    KeyAT      {channel: u8, key: u8, pressure: u8},
    ControlChg {channel: u8, controller: u8, value: u8},
    ProgramChg {channel: u8, program: u8},
    ChannelAT  {channel: u8, pressure: u8},
    Pitchbend  {channel: u8, pitch: i16},
}

struct Config {
    in_port: usize,
    in_channel: u8,
    out_port: usize,
    out_channel: u8,
}

struct Colors {
    c_normal: &'static str,
    c_param: &'static str,
    c_value: &'static str,
}

const COLORS_BW: Colors = Colors{ c_normal: "", c_param: "", c_value: "" };
const COLORS_TC: Colors = Colors{ c_normal: "\x1b[30m", c_param: "\x1b[32m", c_value:"\x1b[34m" };

fn main() {
    let mut config = Config{
        in_port: std::usize::MAX,
        in_channel: 0,
        out_port: std::usize::MAX,
        out_channel: 0,
    };

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

    match receive_data(&configs, monitor, record, outfile, colors) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err)
    }
}

/// Receive data from a MIDI in port and optionally forward it.
///
/// If no output port has been defined, the data is only read, written to file
/// if configured, and written to stdout if configured.
fn receive_data(configs: &[Config], do_monitor: bool, do_record: bool, outfile: &str, colors: &'static Colors)
        -> Result<(), Box<dyn Error>> {

    let mut conn_list = vec!();

    for config in configs {
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

            if message.len() < 2 {
                return; // Unexpected size, ignore
            }
            if in_channel > 0 && (message[0] & 0x0F) != in_channel - 1 {
                return; // Not listening on this channel
            }

            if do_forward {
                // Forward data to configured output port
                if out_channel < 16 && out_channel != in_channel {
                    // Adjust MIDI channel
                    message_out[0] = message[0] & 0xF0 | out_channel - 1;
                } else {
                    message_out[0] = message[0];
                }
                message_out[1] = message[1];
                if message.len() == 3 {
                    message_out[2] = message[2];
                }
                if let Some(c) = conn_out.as_mut() {
                    c.send(&message_out).unwrap_or_else(|_| println!("Error when forwarding message ..."));
                }
            }

            if do_monitor {
                // Print received data to screen
                show_message(timestamp, conf_in_port, message, &colors);
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

fn get_midi_message(message: &[u8]) -> MidiMessage {
    let channel = message[0] & 0x0F;
    let param = message[1];
    let mut value = 0;
    if message.len() > 2 {
        value = message[2];
    }
    match message[0] & 0xF0 {
        0x90 => MidiMessage::NoteOn{channel, key: param, velocity: value},
        0x80 => MidiMessage::NoteOff{channel, key: param, velocity: value},
        0xA0 => MidiMessage::KeyAT{channel, key: param, pressure: value},
        0xB0 => MidiMessage::ControlChg{channel, controller: param, value},
        0xC0 => MidiMessage::ProgramChg{channel, program: param},
        0xD0 => MidiMessage::ChannelAT{channel, pressure: param},
        0xE0 => {
            let mut pitch: i16 = param as i16;
            pitch |= (value as i16) << 7;
            pitch -= 0x2000;
            MidiMessage::Pitchbend{channel, pitch}
        },
        _ => panic!("Cannot convert message {:?}", message),
    }
}

fn show_message(timestamp: u64, in_port: usize, message: &[u8], colors: &Colors) {
    print!("{} Port {} ", timestamp, in_port);
    let m = get_midi_message(message);
    match m {
        MidiMessage::NoteOn{channel, key, velocity} => {
            print!("Ch {} {}NoteOn {}key={} velocity={}",
                channel + 1, colors.c_param, colors.c_value, key, velocity);
        }
        MidiMessage::NoteOff{channel, key, velocity} => {
            print!("Ch {} {}NoteOff {}key={} velocity={}",
                channel + 1, colors.c_param, colors.c_value, key, velocity);
        }
        MidiMessage::KeyAT{channel, key, pressure} => {
            print!("Ch {} {}Aftertouch {}key={} pressure={}",
                channel + 1, colors.c_param, colors.c_value, key, pressure);
        }
        MidiMessage::ControlChg{channel, controller, value} => {
            print!("Ch {} {}ControlChg {}controller={} value={}",
                channel + 1, colors.c_param, colors.c_value, controller, value);
        }
        MidiMessage::ProgramChg{channel, program} => {
            print!("Ch {} {}ProgramChg {}program={}",
                channel + 1, colors.c_param, colors.c_value, program);
        }
        MidiMessage::ChannelAT{channel, pressure} => {
            print!("Ch {} {}ChannelAftertouch {}pressure={}",
                channel + 1, colors.c_param, colors.c_value, pressure);
        }
        MidiMessage::Pitchbend{channel, pitch} => {
            print!("Ch {} {}Pitchbend {}pitch={}",
                channel + 1, colors.c_param, colors.c_value, pitch);
        }
    }
    println!("{}", colors.c_normal);
}
