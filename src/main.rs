extern crate priority_queue;
#[macro_use]
extern crate serde_derive;
extern crate pitch_calc;
extern crate bincode;
extern crate ghakuf;
extern crate sample;
extern crate synth;
extern crate rodio;
extern crate clap;

mod convert_duration;
mod packet;
mod server;
mod client;
mod beep;
mod midi;

use clap::{Arg, App, AppSettings, SubCommand};

use server::server;
use client::client;

fn main() {
    let matches = App::new("midi-orchestra-rs")
        .version("0.1")
        .about("A silly distributed MIDI player nobody asked for or needed!")
        .author("Oliver Maskery")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("server")
            .about("reads MIDI files and orchestrates clients to play it")
            .arg(Arg::with_name("midi")
                .required(true)
                .help("path to the midi file to play")))
        .subcommand(SubCommand::with_name("client")
            .about("connects to a server and dutifully plays note on command")
            .arg(Arg::with_name("target")
                .required(true)
                .help("hostname and port combination of the server to connect to")))
        .get_matches();

    match matches.subcommand() {
        ("server", Some(matches)) => server(matches),
        ("client", Some(matches)) => client(matches),
        (command, _) => panic!("unknown command: {}", command),
    }
}
