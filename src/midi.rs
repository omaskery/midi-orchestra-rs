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

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum MusicalEvent {
    PlayNote {
        channel: u8,
        note: u8,
        start: u64,
        duration: u64,
    },
    ChangeTempo {
        new_tempo: u32,
        start: u64,
    },
}

pub struct Handler {
    handled: u64,
    division: f64,
    current_time: u64,
    book_keeping: HashMap<(u8, u8), u64>,
    music: PriorityQueue<MusicalEvent, u64>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            handled: 0,
            division: 0f64,
            current_time: 0,
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

    fn note_begun(&mut self, channel: u8, note: u8) {
        let key = (channel, note);
        self.book_keeping.insert(key, self.current_time);
    }

    fn note_ended(&mut self, channel: u8, note: u8) {
        let key = (channel, note);
        if self.book_keeping.contains_key(&key) {
            let start = *self.book_keeping.get(&key).unwrap();
            let played = MusicalEvent::PlayNote {
                note,
                channel,
                start,
                duration: self.current_time - start,
            };
            self.music.push(played, start);
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
                    println!("{:>4} [meta] delta_time: {}, tempo: {} ({:?})", self.handled, delta_time, tempo, data);
                } else {
                    println!("{:>4} [meta] delta_time: {}, event: {}, data: {:?} - data length isn't 3!?", self.handled, delta_time, event, data);
                }
            },
            _ => {
                println!("{:>4} [meta] delta_time: {}, event: {}, data: {:?}", self.handled, delta_time, event, data);
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

    fn sys_ex_event(&mut self, delta_time: u32, event: &SysExEvent, data: &Vec<u8>) {
        self.handled += 1;
        println!("{:>4} [sys_ex] delta_time: {}, event: {}, data: {:?}", self.handled, delta_time, event, data);
        self.advance_time(delta_time);
    }

    fn track_change(&mut self) {
        self.handled += 1;
        println!("{:>4} [track_change] resetting current time", self.handled);
        self.current_time = 0;
    }
}

