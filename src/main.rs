extern crate priority_queue;
#[macro_use]
extern crate serde_derive;
extern crate pitch_calc;
extern crate term_size;
extern crate bincode;
extern crate ghakuf;
extern crate sample;
extern crate synth;
extern crate rodio;
extern crate clap;
extern crate pbr;

mod convert_duration;
mod packet;
mod server;
mod client;
mod beep;
mod midi;

use clap::{Arg, App, AppSettings, SubCommand};

use server::server;
use client::client;

// TODO: - refactor so that midi code has tempo handling baked in,
//         that way the subsequent systems can just manipulate musical events
//         without constantly juggling time concerns
//       - then, based on that, can use the include/exclude track/channel
//         arguments to simply filter what events go forward into the actual
//         playback loop
//       - once that's done, look into assigning frequency ranges to clients
//         by placing all notes into a histogram (x-axis = frequency) and
//         giving each client an equal "area under the graph" of adjacent
//         buckets, which should now be possible since all musical events
//         (after filtering) are now available
//       - make channel/track filtering collect<>() using hashset or something
//         rather than doing all that contains() checking with a Vec
//       - make channel/track filter arguments build hashset of legal
//         tracks/channels and simply check those during note playing rather
//         than obtuse logic around each list being present etc.

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
            .arg(Arg::with_name("volume")
                .short("v")
                .long("volume")
                .default_value("1.0")
                .help("coefficient to multiply note volumes by"))
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
