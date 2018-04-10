extern crate pitch_calc;
extern crate ghakuf;
extern crate cpal;

mod beep;

use pitch_calc::{Step, Hz};
use std::time::Duration;
use std::thread::sleep;

use beep::Beeper;

fn main() {
    let beeper = Beeper::new();

    let step_a: Step = Hz(440.0).to_step();
    let step_b: Step = Hz(880.0).to_step();
    let duration = Duration::from_millis(1000);

    println!("start");
    for index in 0..3 {
        println!("  repetition {}", index + 1);
        beep(&beeper, step_a, duration);
        beep(&beeper, step_b, duration);
    }
    println!("stop");
}

fn beep<H: Into<Hz>>(beeper: &Beeper, frequency: H, duration: Duration) {
    beeper.beep(frequency, duration);
    sleep(duration);
}

