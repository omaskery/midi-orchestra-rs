use convert_duration::*;
use packet::Packet;
use beep::Beeper;

use std::time::Duration;
use std::net::TcpStream;
use std::thread::sleep;
use std;

use bincode::{serialize_into, deserialize_from};
use pitch_calc::Hz;
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
                    sleep(Duration::from_secs(1));
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
                beeper.beep(Hz(frequency), nanoseconds_to_duration(duration), volume);
                println!("beep @ {} for {}ns (volume={})", frequency, duration, volume);
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
