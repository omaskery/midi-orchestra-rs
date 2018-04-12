extern crate priority_queue;
extern crate pitch_calc;
extern crate ghakuf;
extern crate sample;
extern crate synth;
extern crate rodio;

mod beep;
mod midi;

use std::time::{Duration, Instant};
use std::thread::sleep;
use pitch_calc::Step;

use midi::MusicalEvent;
use beep::Beeper;

const ONE_SECOND_NS: u64 = 1_000_000_000;

fn main() {
    // let path = std::path::Path::new("midis/Mario-Sheet-Music-Overworld-Main-Theme.mid");
    let path = std::path::Path::new("midis/main-theme.mid");

    let (division, music) = midi::load_midi(path);

    let beeper = Beeper::new();

    let mut last_offset = 0;
    let mut last_note = Instant::now();
    let mut tempo = 500_000f64;

    let clocks_to_duration = |tempo: f64, clocks: u64| {
        // let seconds = (60 * clocks) as f64 / (tempo * division);
        let seconds = ((clocks as f64) * 2.2) / 1_000.0;
        seconds_to_duration(seconds)
    };

    for event in music {
        let start = match &event {
            &MusicalEvent::ChangeTempo { start, .. } => start,
            &MusicalEvent::PlayNote { start, .. } => start,
        };
        let clocks = start - last_offset;
        let event_offset = clocks_to_duration(tempo, clocks);
        last_offset = start;

        let event_time = last_note + event_offset;
        let now = Instant::now();
        last_note += event_offset;

        if now < event_time {
            let time_until_note = event_time - now;
            println!("sleeping for {:?}", time_until_note);
            sleep(time_until_note);
        }

        match event {
            MusicalEvent::ChangeTempo { new_tempo, .. } => {
                println!("tempo changed to {}", new_tempo);
                tempo = new_tempo as f64;
            },
            MusicalEvent::PlayNote { channel, note, duration, .. } => {
                let note = Step(note as f32);
                let duration = clocks_to_duration(tempo, duration);
                beeper.beep(note, duration);
                println!("[{}] beep at {:?} for {:?}", channel, note.to_letter_octave(), duration);
            },
        }
    }
}

fn seconds_to_duration(seconds: f64) -> Duration {
    nanoseconds_to_duration((seconds * (ONE_SECOND_NS as f64)) as u64)
}

fn nanoseconds_to_duration(mut nanoseconds: u64) -> Duration {
    let mut seconds = 0;
    while nanoseconds >= ONE_SECOND_NS {
        nanoseconds -= ONE_SECOND_NS;
        seconds += 1;
    }

    Duration::new(seconds, nanoseconds as u32)
}

