use convert_duration::*;
use packet::Packet;
use beep::Beeper;

use std::time::Duration;
use std::net::TcpStream;
use std::thread::sleep;
use std;

use bincode::{serialize_into, deserialize_from};
use pitch_calc::{Hz, LetterOctave};
use clap::ArgMatches;

pub fn client(matches: &ArgMatches) {
    let forever: bool = matches.is_present("forever");

    if forever == false {
        client_impl(matches).ok();
    } else {
        println!("running forever...");
        loop {
            match client_impl(matches) {
                Ok(_) => {},
                Err(e) => {
                    println!("error: {}", e);
                    sleep(Duration::from_millis(500));
                }
            }

            println!("automatically running again!");
        }
    }
}

fn client_impl(matches: &ArgMatches) -> Result<(), Box<std::error::Error>> {
    let target = matches.value_of("target").unwrap();

    println!("connecting to {}...", target);
    let client = loop {
        let result = TcpStream::connect(target);
        if let Ok(stream) = result {
            break stream;
        }
    };

    println!("sending client info...");
    let info = Packet::ClientInfo;
    serialize_into(&client, &info)?;

    let beeper = Beeper::new();

    println!("awaiting commands...");
    loop {
        let packet: Packet = deserialize_from(&client)?;

        match &packet {
            &Packet::PlayNote { duration, frequency, volume } => {
                let frequency = Hz(frequency);
                let duration = nanoseconds_to_duration(duration);
                beeper.beep(frequency, duration, volume);
                let LetterOctave(letter, octave) = frequency.to_letter_octave();
                let duration_ms = (duration_to_seconds(duration) * 1000f64) as u64;
                println!("beep [{:4} {}] for {:04}ms (volume={:0.2})", format!("{:?},", letter), octave, duration_ms, volume);
            },
            &Packet::TerminateAfter(duration) => {
                println!("terminating after {}ns", duration);
                sleep(nanoseconds_to_duration(duration));
                break;
            }
            packet if packet.is_client_message() => panic!("received client message from server?"),
            packet => println!("unhandled packet: {:?}", packet),
        }
    }

    Ok(())
}
