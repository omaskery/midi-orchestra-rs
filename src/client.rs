use convert_duration::*;
use packet::Packet;
use beep::Beeper;

use std::net::TcpStream;
use std::thread::sleep;

use bincode::{serialize_into, deserialize_from};
use pitch_calc::Hz;
use clap::ArgMatches;

pub fn client(matches: &ArgMatches) {
    let target = matches.value_of("target").unwrap();

    println!("connecting to {}...", target);
    let client = TcpStream::connect(target)
        .expect("failed to connect to host");

    println!("sending client info...");
    let info = Packet::ClientInfo;
    serialize_into(&client, &info)
        .expect("failed to send client info");

    let beeper = Beeper::new();

    println!("awaiting commands...");
    loop {
        let packet: Packet = deserialize_from(&client)
            .expect("failed to deserialise packet");

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
}
