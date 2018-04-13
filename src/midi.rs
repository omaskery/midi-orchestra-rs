use std::collections::HashMap;
use std::path::Path;

use priority_queue::PriorityQueue;
use ghakuf::{messages::*, reader::Reader};
use ghakuf;

pub fn load_midi<P: AsRef<Path>>(path: P) -> (f64, Vec<MusicalEvent>) {
    let mut handler = Handler::new();

    {
        let mut midi_reader = Reader::new(
            &mut handler,
            path.as_ref(),
        ).unwrap();

        let _ = midi_reader.read();
    }

    (handler.get_division(), handler.into_music())
}

#[derive(Copy, Clone)]
struct StartOfNote {
    start: u64,
    velocity: u8,
}

#[derive(Copy, Clone, Debug)]
enum InstrumentFamily {
    Piano,
    ChromaticPercussion,
    Organ,
    Guitar,
    Bass,
    Strings,
    Ensemble,
    Brass,
    Reed,
    Pipe,
    SynthLead,
    SynthPad,
    SynthEffects,
    Ethnic,
    Percussive,
    SoundEffects,
}

impl InstrumentFamily {
    fn from_program(program: u8) -> InstrumentFamily {
        match (program >> 3) & 0xF {
            0x0 => InstrumentFamily::Piano,
            0x1 => InstrumentFamily::ChromaticPercussion,
            0x2 => InstrumentFamily::Organ,
            0x3 => InstrumentFamily::Guitar,
            0x4 => InstrumentFamily::Bass,
            0x5 => InstrumentFamily::Strings,
            0x6 => InstrumentFamily::Ensemble,
            0x7 => InstrumentFamily::Brass,
            0x8 => InstrumentFamily::Reed,
            0x9 => InstrumentFamily::Pipe,
            0xA => InstrumentFamily::SynthLead,
            0xB => InstrumentFamily::SynthPad,
            0xC => InstrumentFamily::SynthEffects,
            0xD => InstrumentFamily::Ethnic,
            0xE => InstrumentFamily::Percussive,
            0xF => InstrumentFamily::SoundEffects,
            _ => panic!("unknown instrument family"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum MusicalEvent {
    PlayNote {
        track: usize,
        channel: u8,
        note: u8,
        start: u64,
        duration: u64,
        velocity: u8,
    },
    ChangeTempo {
        new_tempo: u32,
        start: u64,
    },
    ChangeTimeSignature {
        numerator: u8,
        denominator_exponent: u8,
        start: u64,
    },
}

pub struct Handler {
    handled: u64,
    division: f64,
    current_time: u64,
    current_track: usize,
    book_keeping: HashMap<(u8, u8), StartOfNote>,
    music: PriorityQueue<MusicalEvent, u64>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            handled: 0,
            division: 0f64,
            current_time: 0,
            current_track: 0,
            book_keeping: HashMap::new(),
            music: PriorityQueue::new(),
        }
    }

    pub fn get_division(&self) -> f64 {
        self.division
    }

    pub fn into_music(self) -> Vec<MusicalEvent> {
        let mut music = self.music.into_sorted_vec();
        music.reverse();
        music
    }

    fn advance_time(&mut self, delta_time: u32) {
        self.current_time += delta_time as u64;
    }

    fn set_tempo(&mut self, new_tempo: u32) {
        self.music.push(MusicalEvent::ChangeTempo {
            new_tempo,
            start: self.current_time,
        }, self.current_time);
    }

    fn set_time_signature(&mut self, numerator: u8, denominator_exponent: u8) {
        self.music.push(MusicalEvent::ChangeTimeSignature {
            numerator,
            denominator_exponent,
            start: self.current_time,
        }, self.current_time);
    }

    fn note_begun(&mut self, channel: u8, note: u8, velocity: u8) {
        let key = (channel, note);
        if self.book_keeping.contains_key(&key) {
            self.note_ended(channel, note);
        }
        self.book_keeping.insert(key, StartOfNote {
            start: self.current_time,
            velocity,
        });
    }

    fn note_ended(&mut self, channel: u8, note: u8) {
        let key = (channel, note);
        if self.book_keeping.contains_key(&key) {
            let start_of_note = *self.book_keeping.get(&key).unwrap();
            let played = MusicalEvent::PlayNote {
                track: self.current_track,
                note,
                channel: channel + 1, // remember that from in MIDI channels are 1-indexed
                start: start_of_note.start,
                duration: self.current_time - start_of_note.start,
                velocity: start_of_note.velocity,
            };
            self.music.push(played, start_of_note.start);
            self.book_keeping.remove(&key);
        }
    }
}

impl ghakuf::reader::Handler for Handler {
    fn header(&mut self, format: u16, track: u16, time_base: u16) {
        self.handled += 1;
        self.division = time_base as f64;
        println!("{:>4} [header] format: {}, track: {}, time_base: {}", self.handled, format, track, time_base);
    }

    fn meta_event(&mut self, delta_time: u32, event: &MetaEvent, data: &Vec<u8>) {
        self.handled += 1;
        match event {
            &MetaEvent::SetTempo => {
                if data.len() == 3 {
                    let tempo = ((data[0] as u32) << 16)
                        | ((data[1] as u32) << 8)
                        | (data[2] as u32);
                    self.set_tempo(tempo);
                    // println!("{:>4} [meta] tempo: {} ({:?})", self.handled, tempo, data);
                } else {
                    println!("{:>4} [meta] event: {}, data: {:?} - data length isn't 3!?", self.handled, event, data);
                }
            },

            &MetaEvent::TimeSignature => {
                if data.len() == 4 {
                    let numerator = data[0];
                    let denominator_exponent = data[1];
                    self.set_time_signature(numerator, denominator_exponent);
                    // println!("{:>4} [meta] time signature: {}/2^{} ({:?})", self.handled, numerator, denominator_exponent, data);
                } else {
                    println!("{:>4} [meta] event: {}, data: {:?} - data length isn't 4!?", self.handled, event, data);
                }
            }

            &MetaEvent::TextEvent => {
                // println!("{:>4} [meta] text event: {}", self.handled, slice_to_text(data));
            }

            &MetaEvent::CopyrightNotice => {
                // println!("{:>4} [meta] copyright: {}", self.handled, slice_to_text(data));
            }

            &MetaEvent::SequenceOrTrackName => {
                println!("{:>4} [meta] seq/track name: {}", self.handled, slice_to_text(data));
            }

            &MetaEvent::MIDIChannelPrefix => {
                if data.len() == 1 {
                    let prefix = data[0];
                    println!("{:>4} [meta] MIDI channel prefix: {}", self.handled, prefix + 1);
                } else {
                    println!("{:>4} [meta] event: {}, data: {:?} - data length isn't 1!?", self.handled, event, data);
                }
            }

            &MetaEvent::KeySignature => {
                // don't currently care about this
            }

            &MetaEvent::EndOfTrack => {
                // println!("{:>4} [meta] END OF TRACK", self.handled);
            }

            &MetaEvent::Unknown { event_type, .. } => {
                // fed up of seeing unknown message 33 - idk what it is
                if event_type != 33 {
                    println!("{:>4} [meta] unknown event: {}, data: {:?}", self.handled, event_type, data);
                }
            }

            _ => {
                println!("{:>4} [meta] event: {}, data: {:?}", self.handled, event, data);
            },
        }
        self.advance_time(delta_time);
    }

    fn midi_event(&mut self, delta_time: u32, event: &MidiEvent) {
        self.handled += 1;
        self.advance_time(delta_time);

        match event {
            &MidiEvent::NoteOn { ch, note, velocity } => {
                if velocity != 0 {
                    self.note_begun(ch, note, velocity);
                } else {
                    self.note_ended(ch, note);
                }
            }

            &MidiEvent::NoteOff { ch, note, .. } => {
                self.note_ended(ch, note);
            },

            &MidiEvent::ProgramChange { ch, program } => {
                println!("{:>4} [midi] program change [channel {}]: {} ({:?})", self.handled, ch + 1, program + 1, InstrumentFamily::from_program(program));
            }

            &MidiEvent::ControlChange { .. } => {
                // currently don't care about this
            },

            &MidiEvent::PitchBendChange { .. } => {
                // currently don't care about this
            },

            _ => {
                println!("{:>4} [midi] event: {}", self.handled, event);
            },
        }
    }

    fn sys_ex_event(&mut self, delta_time: u32, event: &SysExEvent, data: &Vec<u8>) {
        self.handled += 1;
        println!("{:>4} [sys_ex] event: {}, data: {:?}", self.handled, event, data);
        self.advance_time(delta_time);
    }

    fn track_change(&mut self) {
        self.handled += 1;
        // println!("{:>4} [track_change] resetting current time", self.handled);
        self.current_time = 0;
        self.current_track += 1;
    }
}

fn slice_to_text(text: &[u8]) -> String {
    String::from_utf8(text.to_vec()).unwrap_or("<failed to decode text>".into())
}

