use std::collections::HashMap;
use std::time::Duration;
use std::path::Path;

use priority_queue::PriorityQueue;
use ghakuf::{messages, messages::{MetaEvent, SysExEvent}, reader::Reader};
use ghakuf;

use convert_duration::seconds_to_duration;

#[derive(Clone, Debug)]
pub struct Timing {
    pub ticks_per_quarter_note: f64,
    pub microseconds_per_quarter_note: f64,
    pub time_signature_numerator: f64,
    pub time_signature_denominator: f64,
}

pub fn clocks_to_duration(timing: &Timing, clocks: Ticks) -> Duration {
    let seconds_per_quarter_note = timing.microseconds_per_quarter_note / 1_000_000.0;
    let seconds_per_tick = seconds_per_quarter_note / timing.ticks_per_quarter_note;
    let seconds = clocks.0 as f64 * seconds_per_tick;
    seconds_to_duration(seconds)
}

pub struct Music {
    events: Vec<MusicalEvent>,
}

impl Music {
    pub fn events(&self) -> &[MusicalEvent] {
        &self.events
    }
}

pub fn load_midi<P: AsRef<Path>>(path: P, verbose: bool) -> Music {
    let mut handler = Handler::new(verbose);

    {
        let mut midi_reader = Reader::new(
            &mut handler,
            path.as_ref(),
        ).unwrap();

        let _ = midi_reader.read();
    }

    let division = handler.get_division();
    let midi = handler.into_music();

    let mut last_start_tick = Ticks(0);
    let mut offset_of_last_event = Duration::new(0, 0);
    let mut timing = Timing {
        ticks_per_quarter_note: division,
        microseconds_per_quarter_note: 500_000.0,
        time_signature_numerator: 4.0,
        time_signature_denominator: 4.0,
    };

    let mut events = Vec::new();

    for event in midi {
        let start_tick = match event {
            MidiEvent::PlayNote { start, .. } => start,
            MidiEvent::ChangeTempo { start, .. } => start,
            MidiEvent::ChangeTimeSignature { start, .. } => start,
        };

        let delta_ticks = Ticks(start_tick.0 - last_start_tick.0);
        let delta_time = clocks_to_duration(&timing, delta_ticks);
        last_start_tick = start_tick;

        let start_offset = offset_of_last_event + delta_time;
        offset_of_last_event = start_offset;

        match event {
            MidiEvent::PlayNote { track, channel, note, duration, velocity, .. } => {
                let duration = clocks_to_duration(&timing, duration);
                events.push(MusicalEvent::PlayNote(Note {
                    start_offset,
                    track,
                    channel,
                    note,
                    duration,
                    velocity,
                }));
            },
            MidiEvent::ChangeTempo { new_tempo, .. } => {
                timing.microseconds_per_quarter_note = new_tempo as f64;
                events.push(MusicalEvent::TimingChange(TimingChange {
                    start_offset,
                    timing: timing.clone(),
                }));
            },
            MidiEvent::ChangeTimeSignature { numerator, denominator_exponent, .. } => {
                let numerator = numerator as f64;
                let denominator = 2.0f64.powf(denominator_exponent as f64);
                timing.time_signature_numerator = numerator as f64;
                timing.time_signature_denominator = denominator;
                events.push(MusicalEvent::TimingChange(TimingChange {
                    start_offset,
                    timing: timing.clone(),
                }));
            },
        }
    }

    Music {
        events,
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Ticks(u64);

#[derive(Clone, Debug)]
pub struct Note {
    pub start_offset: Duration,
    pub channel: u8,
    pub track: usize,
    pub note: u8,
    pub duration: Duration,
    pub velocity: u8,
}

#[derive(Clone, Debug)]
pub struct TimingChange {
    pub start_offset: Duration,
    pub timing: Timing,
}

#[derive(Clone, Debug)]
pub enum MusicalEvent {
    PlayNote(Note),
    TimingChange(TimingChange),
}

#[derive(Copy, Clone)]
struct StartOfNote {
    start: Ticks,
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
pub enum MidiEvent {
    PlayNote {
        track: usize,
        channel: u8,
        note: u8,
        start: Ticks,
        duration: Ticks,
        velocity: u8,
    },
    ChangeTempo {
        new_tempo: u32,
        start: Ticks,
    },
    ChangeTimeSignature {
        numerator: u8,
        denominator_exponent: u8,
        start: Ticks,
    },
}

pub struct Handler {
    verbose: bool,
    handled: u64,
    division: f64,
    current_time: Ticks,
    current_track: usize,
    book_keeping: HashMap<(u8, u8), StartOfNote>,
    events: PriorityQueue<MidiEvent, Ticks>,
}

impl Handler {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            handled: 0,
            division: 0f64,
            current_time: Ticks(0),
            current_track: 0,
            book_keeping: HashMap::new(),
            events: PriorityQueue::new(),
        }
    }

    pub fn get_division(&self) -> f64 {
        self.division
    }

    pub fn into_music(self) -> Vec<MidiEvent> {
        let mut music = self.events.into_sorted_vec();
        music.reverse();
        music
    }

    fn advance_time(&mut self, delta_time: u32) {
        self.current_time = Ticks(self.current_time.0 + delta_time as u64);
    }

    fn set_tempo(&mut self, new_tempo: u32) {
        self.events.push(MidiEvent::ChangeTempo {
            new_tempo,
            start: self.current_time,
        }, self.current_time);
    }

    fn set_time_signature(&mut self, numerator: u8, denominator_exponent: u8) {
        self.events.push(MidiEvent::ChangeTimeSignature {
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
            let played = MidiEvent::PlayNote {
                track: self.current_track,
                note,
                channel: channel + 1, // remember that from in MIDI channels are 1-indexed
                start: start_of_note.start,
                duration: Ticks(self.current_time.0 - start_of_note.start.0),
                velocity: start_of_note.velocity,
            };
            self.events.push(played, start_of_note.start);
            self.book_keeping.remove(&key);
        }
    }
}

impl ghakuf::reader::Handler for Handler {
    fn header(&mut self, format: u16, track: u16, time_base: u16) {
        self.handled += 1;
        self.division = time_base as f64;
        if self.verbose {
            println!("{:>4} [header] format: {}, track: {}, time_base: {}", self.handled, format, track, time_base);
        }
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
                if self.verbose {
                    println!("{:>4} [meta] seq/track name: {}", self.handled, slice_to_text(data));
                }
            }

            &MetaEvent::MIDIChannelPrefix => {
                if data.len() == 1 {
                    let prefix = data[0];
                    if self.verbose {
                        println!("{:>4} [meta] MIDI channel prefix: {}", self.handled, prefix + 1);
                    }
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
                if self.verbose {
                    println!("{:>4} [meta] event: {}, data: {:?}", self.handled, event, data);
                }
            },
        }
        self.advance_time(delta_time);
    }

    fn midi_event(&mut self, delta_time: u32, event: &messages::MidiEvent) {
        self.handled += 1;
        self.advance_time(delta_time);

        match event {
            &messages::MidiEvent::NoteOn { ch, note, velocity } => {
                if velocity != 0 {
                    self.note_begun(ch, note, velocity);
                } else {
                    self.note_ended(ch, note);
                }
            }

            &messages::MidiEvent::NoteOff { ch, note, .. } => {
                self.note_ended(ch, note);
            },

            &messages::MidiEvent::ProgramChange { ch, program } => {
                if self.verbose {
                    println!("{:>4} [midi] program change [channel {}]: {} ({:?})", self.handled, ch + 1, program + 1, InstrumentFamily::from_program(program));
                }
            }

            &messages::MidiEvent::ControlChange { .. } => {
                // currently don't care about this
            },

            &messages::MidiEvent::PitchBendChange { .. } => {
                // currently don't care about this
            },

            _ => {
                if self.verbose {
                    println!("{:>4} [midi] event: {}", self.handled, event);
                }
            },
        }
    }

    fn sys_ex_event(&mut self, delta_time: u32, event: &SysExEvent, data: &Vec<u8>) {
        self.handled += 1;
        if self.verbose {
            println!("{:>4} [sys_ex] event: {}, data: {:?}", self.handled, event, data);
        }
        self.advance_time(delta_time);
    }

    fn track_change(&mut self) {
        self.handled += 1;
        // println!("{:>4} [track_change] resetting current time", self.handled);
        self.current_time = Ticks(0);
        self.current_track += 1;
    }
}

fn slice_to_text(text: &[u8]) -> String {
    String::from_utf8(text.to_vec()).unwrap_or("<failed to decode text>".into())
}

