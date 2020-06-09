use super::Avg;
use super::MidiMessage;

pub struct Colors {
    c_normal: &'static str,
    c_param: &'static str,
    c_value: &'static str,
}

pub const COLORS_BW: Colors = Colors{ c_normal: "", c_param: "", c_value: "" };
pub const COLORS_TC: Colors = Colors{ c_normal: "\x1b[30m", c_param: "\x1b[32m", c_value:"\x1b[34m" };

pub struct Display {
    colors: &'static Colors,
    show_time: bool,
    bpm: f64,
    last_clock: u64, // Timestamp of last TimingClock message (usec)
    avg: Avg,
}

impl Display {
    pub fn new(colors: &'static Colors, show_time: bool) -> Self {
        Display{
            colors,
            show_time,
            bpm: 0.0,
            last_clock: 0,
            avg: Avg::new(48), // Average over 2 quarters (2 * 24 timestamps)
        }
    }

    fn print_tpc(&self, timestamp: u64, port: usize, channel: u8) {
        print!("{} Port {} Ch {} {}", timestamp, port, channel, self.colors.c_param);
    }

    fn print_tp(&self, timestamp: u64, port: usize) {
        print!("{} Port {} {}", timestamp, port, self.colors.c_param);
    }

    fn print_footer(&self) {
        println!("{}", self.colors.c_normal);
    }

    fn calc_bpm(&mut self, timestamp: u64) {
        if self.last_clock != 0 {
            // We have a previous TS, so we can calculate the current BPM
            let diff = (timestamp - self.last_clock) * 24; // Diff is in usec
            let bpm = 60000000.0 / diff as f64;
            let result = self.avg.add_value(bpm);
            match result {
                Some(bpm) => {
                    // Calculate up to 1 decimal of BPM
                    let bpm = (bpm * 10.0).round() / 10.0;
                    if bpm != self.bpm {
                        println!("{} BPM {}", timestamp, bpm);
                        self.bpm = bpm;
                    }
                }
                None => ()
            }
        }
        self.last_clock = timestamp;
    }

    pub fn show_message(&mut self, timestamp: u64, in_port: usize, message: &[u8]) {
        let m = MidiMessage::parse(message);
        match m {
            MidiMessage::NoteOn{channel, key, velocity} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("NoteOn {}key={} velocity={}", self.colors.c_value, key, velocity);
            }
            MidiMessage::NoteOff{channel, key, velocity} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("NoteOff {}key={} velocity={}", self.colors.c_value, key, velocity);
            }
            MidiMessage::KeyAT{channel, key, pressure} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("Aftertouch {}key={} pressure={}", self.colors.c_value, key, pressure);
            }
            MidiMessage::ControlChg{channel, controller, value} => {
                // TODO: Show channel mode messages (120 - 127)
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("ControlChg {}controller={} value={}", self.colors.c_value, controller, value);
            }
            MidiMessage::ProgramChg{channel, program} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("ProgramChg {}program={}", self.colors.c_value, program);
            }
            MidiMessage::ChannelAT{channel, pressure} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("ChannelAftertouch {}pressure={}", self.colors.c_value, pressure);
            }
            MidiMessage::Pitchbend{channel, pitch} => {
                self.print_tpc(timestamp, in_port, channel + 1);
                print!("Pitchbend {}pitch={}", self.colors.c_value, pitch);
            }
            MidiMessage::SongPos{position} => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("SongPosition {}position={}", self.colors.c_value, position);
            }
            MidiMessage::TimingClock => {
                if !self.show_time { return; }
                self.calc_bpm(timestamp);
                return;
                //self.print_tp(timestamp, in_port);
                //print!("TimingClock");
            }
            MidiMessage::Start => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("Start");
            }
            MidiMessage::Continue => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("Continue");
            }
            MidiMessage::Stop => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("Stop");
            }
            MidiMessage::ActiveSensing => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("ActiveSensing");
            }
            MidiMessage::Reset => {
                if !self.show_time { return; }
                self.print_tp(timestamp, in_port);
                print!("Reset");
            }
        }
        self.print_footer();
    }
}
