extern crate pitch_calc;
extern crate ghakuf;
extern crate sample;
extern crate synth;
extern crate rodio;

mod beep;

use std::collections::HashMap;
use pitch_calc::Step;
use std::time::{Duration, Instant};
use ghakuf::messages::*;
use std::thread::sleep;

use beep::Beeper;

#[derive(Debug)]
pub struct PlayedNote {
    channel: u8,
    note: u8,
    start: u64,
    end: u64,
}

impl PlayedNote {
    pub fn duration(&self) -> u64 {
        self.end - self.start
    }
}

pub struct Handler {
    handled: u64,
    current_time: u64,
    book_keeping: HashMap<(u8, u8), u64>,
    notes: Vec<PlayedNote>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            handled: 0,
            current_time: 0,
            book_keeping: HashMap::new(),
            notes: Vec::new(),
        }
    }

    pub fn into_notes(self) -> Vec<PlayedNote> {
        self.notes
    }

    fn advance_time(&mut self, delta_time: u32) {
        self.current_time += delta_time as u64;
    }

    fn note_begun(&mut self, channel: u8, note: u8) {
        let key = (channel, note);
        self.book_keeping.insert(key, self.current_time);
    }

    fn note_ended(&mut self, channel: u8, note: u8) {
        let key = (channel, note);
        if self.book_keeping.contains_key(&key) {
            let start = *self.book_keeping.get(&key).unwrap();
            let played = PlayedNote {
                note,
                channel,
                start,
                end: self.current_time,
            };
            self.notes.push(played);
            self.book_keeping.remove(&key);
        }
    }
}

impl ghakuf::reader::Handler for Handler {
    fn header(&mut self, format: u16, track: u16, time_base: u16) {
        self.handled += 1;
        println!("{:>4} [header] format: {}, track: {}, time_base: {}", self.handled, format, track, time_base);
    }

    fn meta_event(&mut self, delta_time: u32, event: &MetaEvent, _data: &Vec<u8>) {
        self.handled += 1;
        println!("{:>4} [meta] delta_time: {}, event: {}", self.handled, delta_time, event);
        // self.advance_time(delta_time);
    }

    fn midi_event(&mut self, delta_time: u32, event: &MidiEvent) {
        self.handled += 1;
        self.advance_time(delta_time);

        match event {
            &MidiEvent::NoteOn { ch, note, velocity } => {
                if velocity != 0 {
                    self.note_begun(ch, note);
                } else {
                    self.note_ended(ch, note);
                }
            }
            &MidiEvent::NoteOff { ch, note, .. } => {
                self.note_ended(ch, note);
            },
            _ => {
                println!("{:>4} [midi] delta_time: {}, event: {}", self.handled, delta_time, event);
            },
        }
    }

    fn sys_ex_event(&mut self, delta_time: u32, event: &SysExEvent, _data: &Vec<u8>) {
        self.handled += 1;
        println!("{:>4} [sys_ex] delta_time: {}, event: {}", self.handled, delta_time, event);
        self.advance_time(delta_time);
    }

    fn track_change(&mut self) {
        self.handled += 1;
        println!("{:>4} [track_change]", self.handled);
    }
}

fn main() {
    let mut handler = Handler::new();
    {
        let path = std::path::Path::new("midis/Mario-Sheet-Music-Overworld-Main-Theme.mid");
        // let path = std::path::Path::new("midis/corneria.mid");
        let mut midi_reader = ghakuf::reader::Reader::new(
            &mut handler,
            &path,
        ).unwrap();

        let _ = midi_reader.read();
    }
    let notes = handler.into_notes();

    let beeper = Beeper::new();
    let scale_time = |t: u64| {
        Duration::from_millis(t / 8)
    };

    let min_channel = notes.iter().map(|p| p.channel).min();
    let max_channel = notes.iter().map(|p| p.channel).max();
    match (min_channel, max_channel) {
        (Some(min), Some(max)) => {
            for channel in min..(max + 1) {
                let notes_in_channel = notes.iter()
                    .filter(|p| p.channel == channel)
                    .collect::<Vec<_>>();
                if notes_in_channel.len() > 0 {
                    let count = notes_in_channel.len();
                    let start = notes_in_channel[0].start;
                    println!("{} notes in channel {} - starts at {:?}", count, channel, scale_time(start));
                }
            }
        },
        _ => {},
    }

    let start_time = Instant::now();
    for played in notes {
        let note_time = start_time + scale_time(played.start);
        let now = Instant::now();

        if now < note_time {
            let time_until_note = note_time - now;
            println!("sleeping for {:?}", time_until_note);
            sleep(time_until_note);
        }

        let note = Step(played.note as f32);
        let duration = scale_time(played.duration());

        beeper.beep(note, duration);
        println!("[{}] beep at {:?} for {:?}", played.channel, note.to_letter_octave(), duration);
    }
}

