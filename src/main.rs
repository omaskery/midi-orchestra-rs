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

struct Timing {
    ticks_per_quarter_note: f64,
    microseconds_per_quarter_note: f64,
    time_signature_numerator: f64,
    time_signature_denominator: f64,
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("no midi path provided");

    let (division, music) = midi::load_midi(path);

    let beeper = Beeper::new();

    let mut last_offset = 0;
    let mut last_note = Instant::now();
    let mut timing = Timing {
        ticks_per_quarter_note: division,
        microseconds_per_quarter_note: 500_000.0,
        time_signature_numerator: 4.0,
        time_signature_denominator: 4.0,
    };

    let clocks_to_duration = |timing: &Timing, clocks: u64| {
        let seconds_per_quarter_note = timing.microseconds_per_quarter_note / 1_000_000.0;
        let seconds_per_tick = seconds_per_quarter_note / timing.ticks_per_quarter_note;
        let seconds = clocks as f64 * seconds_per_tick;
        seconds_to_duration(seconds)
    };

    for event in music {
        let start = match &event {
            &MusicalEvent::PlayNote { start, .. } => start,
            &MusicalEvent::ChangeTempo { start, .. } => start,
            &MusicalEvent::ChangeTimeSignature { start, .. } => start,
        };
        let clocks = start - last_offset;
        let event_offset = clocks_to_duration(&timing, clocks);
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
            MusicalEvent::PlayNote { channel, note, duration, velocity, .. } => {
                let note = Step(note as f32);
                let duration = clocks_to_duration(&timing, duration);
                let volume = velocity as f32 / 128.0;
                beeper.beep(note, duration, volume);
                println!("[{}] beep at {:?} for {:?}", channel, note.to_letter_octave(), duration);
            },
            MusicalEvent::ChangeTempo { new_tempo, .. } => {
                println!("tempo changed to {}", new_tempo);
                timing.microseconds_per_quarter_note = new_tempo as f64;
            },
            MusicalEvent::ChangeTimeSignature { numerator, denominator_exponent, .. } => {
                let numerator = numerator as f64;
                let denominator = 2.0f64.powf(denominator_exponent as f64);
                println!("time signature changed to {}/{}", numerator as u32, denominator as u32);
                timing.time_signature_numerator = numerator as f64;
                timing.time_signature_denominator = denominator;
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

