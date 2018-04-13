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

mod beep;
mod midi;

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::thread::{sleep, spawn};
use std::sync::{Arc, Mutex};
use std::io::Write;

use bincode::{serialize_into, deserialize_from};
use clap::{Arg, ArgMatches, App, AppSettings, SubCommand};
use pitch_calc::{Step, Hz};

use midi::MusicalEvent;
use beep::Beeper;

const ONE_SECOND_NS: u64 = 1_000_000_000;

struct Timing {
    ticks_per_quarter_note: f64,
    microseconds_per_quarter_note: f64,
    time_signature_numerator: f64,
    time_signature_denominator: f64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Packet {
    ClientInfo,
    PlayNote {
        duration: u64,
        frequency: f32,
        volume: f32,
    },
    TerminateAfter(u64),
}

impl Packet {
    pub fn is_client_message(&self) -> bool {
        match self {
            &Packet::ClientInfo => true,
            _ => false,
        }
    }
}

struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn send(&self, packet: Packet) -> Result<(), Box<bincode::ErrorKind>> {
        serialize_into(&self.stream, &packet)
    }

    pub fn recv(&self) -> Result<Packet, Box<bincode::ErrorKind>> {
        deserialize_from(&self.stream)
    }
}

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

fn server(matches: &ArgMatches) {
    let path = matches.value_of("midi").unwrap();

    let listener = TcpListener::bind("0.0.0.0:8000")
        .expect("unable to create TCP server");

    let connections = Arc::new(Mutex::new(Vec::new()));

    let connections_clone = connections.clone();
    spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let mut c = connections_clone.lock()
                        .expect("failed to acquire mutex while accepting");
                    let mut connection = Connection {
                        stream: s,
                    };

                    connection.stream.set_nodelay(true)
                        .expect("failed to set connection to be no-delay");

                    let info = connection.recv()
                        .expect("failed to receive client info packet");
                    let okay = match info {
                        Packet::ClientInfo => {
                            true
                        },
                        _ => false,
                    };

                    if okay {
                        c.push(connection);
                        println!("connection accepted");
                    } else {
                        println!("connection rejected");
                        connection.send(Packet::TerminateAfter(0))
                            .expect("failed to send rejection termination");
                        connection.stream.flush()
                            .expect("failed to flush rejected connection");
                        connection.stream.shutdown(std::net::Shutdown::Both)
                            .expect("failed to shutdown rejected connection");
                    }
                },
                Err(e) => panic!("IO error while listening: {}", e),
            }
        }
    });

    println!("loading midi...");
    let (division, music) = midi::load_midi(path);

    let delay_period = Duration::from_secs(5);
    println!("waiting {:?} seconds for clients to connect...", delay_period);
    sleep(delay_period);

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

    let mut connection_index = 0;

    println!("starting playback!");
    let mut latest_note_end_time = Instant::now();
    for event in music.iter() {
        let start = match event {
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
            &MusicalEvent::PlayNote { channel, note, duration, velocity, .. } => {
                let note = Step(note as f32);
                let duration = clocks_to_duration(&timing, duration);
                let volume = velocity as f32 / 128.0;
                let end_time = now + duration;
                if end_time >= latest_note_end_time {
                    latest_note_end_time = end_time;
                }
                println!("[{}] beep at {:?} for {:?}", channel, note.to_letter_octave(), duration);

                let c = connections.lock()
                    .expect("failed to lock mutex to send note");
                if let Some(client) = c.iter().nth(connection_index) {
                    client.send( Packet::PlayNote {
                        duration: duration_to_nanoseconds(duration),
                        frequency: note.to_hz().0,
                        volume,
                    }).expect("failed to send note packet");
                }
                connection_index = (connection_index + 1) % c.len();
            },
            &MusicalEvent::ChangeTempo { new_tempo, .. } => {
                println!("tempo changed to {}", new_tempo);
                timing.microseconds_per_quarter_note = new_tempo as f64;
            },
            &MusicalEvent::ChangeTimeSignature { numerator, denominator_exponent, .. } => {
                let numerator = numerator as f64;
                let denominator = 2.0f64.powf(denominator_exponent as f64);
                println!("time signature changed to {}/{}", numerator as u32, denominator as u32);
                timing.time_signature_numerator = numerator as f64;
                timing.time_signature_denominator = denominator;
            },
        }
    }

    let now = Instant::now();
    let terminate_delay = if now < latest_note_end_time {
        duration_to_nanoseconds(latest_note_end_time - now)
    } else {
        0
    };

    let mut c = connections.lock()
        .expect("failed to lock mutex for terminate packet");

    println!("telling clients to terminate...");
    for client in c.iter_mut() {
        client.send( Packet::TerminateAfter(
            terminate_delay
        )).expect("failed to serialize termination packet");
    }

    println!("ensuring clients get termination messages...");
    for client in c.iter_mut() {
        client.stream.flush()
            .expect("failed to flush rejected connection");
        client.stream.shutdown(std::net::Shutdown::Both)
            .expect("failed to shutdown client during termination");
    }

    // better safe than sorry!
    if terminate_delay > 0 {
        sleep(nanoseconds_to_duration(terminate_delay));
    }

    println!("done");
}

fn client(matches: &ArgMatches) {
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

fn duration_to_nanoseconds(duration: Duration) -> u64 {
    (duration.as_secs() * ONE_SECOND_NS) + duration.subsec_nanos() as u64
}
