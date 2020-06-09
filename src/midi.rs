
pub enum MidiMessage {
    NoteOff    {channel: u8, key: u8, velocity: u8},
    NoteOn     {channel: u8, key: u8, velocity: u8},
    KeyAT      {channel: u8, key: u8, pressure: u8},
    ControlChg {channel: u8, controller: u8, value: u8},
    ProgramChg {channel: u8, program: u8},
    ChannelAT  {channel: u8, pressure: u8},
    Pitchbend  {channel: u8, pitch: i16},
    SongPos    {position: u16},
    TimingClock,
    Start,
    Continue,
    Stop,
    ActiveSensing,
    Reset,
}

impl MidiMessage {
    pub fn parse(message: &[u8]) -> MidiMessage {
        let channel = message[0] & 0x0F;
        let param = if message.len() > 1 { message[1] } else { 0 };
        let value = if message.len() > 2 { message[2] } else { 0 };

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
            0xF0 => {
                // System Real-Time Messages
                match message[0] {
                    0xF2 => {
                        let mut position: u16 = param as u16;
                        position |= (value as u16) << 7;
                        MidiMessage::SongPos{position}
                    }
                    0xF8 => MidiMessage::TimingClock,
                    0xFA => MidiMessage::Start,
                    0xFB => MidiMessage::Continue,
                    0xFC => MidiMessage::Stop,
                    0xFE => MidiMessage::ActiveSensing,
                    0xFF => MidiMessage::Reset,
                    _ => panic!("Cannot convert message {:?}", message),
                }
            },
            _ => panic!("Cannot convert message {:?}", message),
        }
    }
}
