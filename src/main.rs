extern crate priority_queue;
#[macro_use]
extern crate serde_derive;
extern crate pitch_calc;
extern crate itertools;
extern crate term_size;
extern crate bincode;
extern crate ghakuf;
extern crate sample;
extern crate synth;
extern crate rodio;
extern crate clap;
extern crate pbr;

mod convert_duration;
mod connection;
mod policies;
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
                .help("path to the midi file to play"))
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .default_value("4000")
                .help("port to listen for client connections on"))
            .arg(Arg::with_name("policy")
                .long("policy")
                .default_value("by-freq")
                .help("determines policy used to assign a note to a particular client")
                .possible_values(&[
                    "broadcast",
                    "by-track",
                    "by-channel",
                    "by-freq",
                    "by-freq-spreadX2",
                ]))
            .arg(Arg::with_name("volume")
                .long("volume")
                .default_value("1.0")
                .help("coefficient to multiply note volumes by"))
            .arg(Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("enable verbose output"))
            .arg(Arg::with_name("exclude track")
                .long("exclude-track")
                .value_name("TRACK")
                .multiple(true)
                .conflicts_with("include track")
                .help("marks a track number for exclusion from playback"))
            .arg(Arg::with_name("include track")
                .long("include-track")
                .value_name("TRACK")
                .multiple(true)
                .conflicts_with("exclude track")
                .help("marks a track number for inclusion in playback"))
            .arg(Arg::with_name("exclude channel")
                .long("exclude-channel")
                .value_name("CHANNEL")
                .multiple(true)
                .conflicts_with("include channel")
                .help("marks a channel number for exclusion from playback"))
            .arg(Arg::with_name("include channel")
                .long("include-channel")
                .value_name("CHANNEL")
                .multiple(true)
                .conflicts_with("exclude channel")
                .help("marks a channel number for inclusion in playback"))
            .arg(Arg::with_name("allow channel 10")
                .long("--allow-channel-10")
                .help("channel 10 is ignored as percussion, this flag allows channel 10 to play")))

        .subcommand(SubCommand::with_name("client")
            .about("connects to a server and dutifully plays note on command")
            .arg(Arg::with_name("target")
                .required(true)
                .help("hostname and port combination of the server to connect to"))
            .arg(Arg::with_name("forever")
                .short("f")
                .long("forever")
                .help("causes client to reconnect forever for unattended operation")))

        .get_matches();

    match matches.subcommand() {
        ("server", Some(matches)) => server(matches),
        ("client", Some(matches)) => client(matches),
        (command, _) => panic!("unknown command: {}", command),
    }
}
