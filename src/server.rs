use convert_duration::*;
use midi::MusicalEvent;
use packet::Packet;
use midi;

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::thread::{sleep, spawn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::Write;
use std;

use bincode::{serialize_into, deserialize_from};
use pitch_calc::Step;
use clap::ArgMatches;
use bincode;

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

struct SharedState {
    connections: Vec<Connection>,
    track_assignments: HashMap<usize, usize>,
}

struct Timing {
    ticks_per_quarter_note: f64,
    microseconds_per_quarter_note: f64,
    time_signature_numerator: f64,
    time_signature_denominator: f64,
}

pub fn server(matches: &ArgMatches) {
    let path = matches.value_of("midi").unwrap();
    let port: u16 = match matches.value_of("port").unwrap().parse() {
        Ok(value) => value,
        Err(_) => {
            println!("invalid port value, must be integer between 0-65535 inclusive");
            return;
        },
    };
    let included_tracks = match_number_list(matches, "include track", "track");
    let excluded_tracks = match_number_list(matches, "exclude track", "track");
    let included_channels = match_number_list(matches, "include channel", "channel");
    let mut excluded_channels = match_number_list(matches, "exclude channel", "channel");

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .expect("unable to create TCP server");

    let shared_state_original = Arc::new(Mutex::new(SharedState {
        connections: Vec::new(),
        track_assignments: HashMap::new(),
    }));

    println!("loading midi...");
    let (division, music) = midi::load_midi(path);

    let tracks = music.iter()
        .map(|e| {
            if let &MusicalEvent::PlayNote { track, .. } = e {
                Some(track)
            } else {
                None
            }
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap())
        .fold(Vec::new(), |mut acc, track| {
            if acc.contains(&track) == false {
                acc.push(track);
            }

            acc
        });

    let channels = music.iter()
        .map(|e| {
            if let &MusicalEvent::PlayNote { channel, .. } = e {
                Some(channel)
            } else {
                None
            }
        })
        .filter(|e| e.is_some())
        .map(|e| e.unwrap())
        .fold(Vec::new(), |mut acc, channel| {
            if acc.contains(&channel) == false {
                acc.push(channel);
            }

            acc
        });

    if channels.contains(&10) && matches.is_present("allow channel 10") == false {
        println!("automatically ignoring channel 10");
        println!("  (set --allow-channel-10 to inhibit this)");

        excluded_channels = excluded_channels
            .map_or(Some(vec![10]), |mut channels| {
                if channels.contains(&10) == false {
                    channels.push(10);
                }

                Some(channels)
            });
    }

    println!("tracks: {:?}", tracks);
    println!("channels: {:?}", channels);

    let shared_state = shared_state_original.clone();
    spawn(move || {
        println!("accepting client connections...");
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let mut state = shared_state_original.lock()
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
                        state.connections.push(connection);
                        state.track_assignments = assign_tracks(&tracks, state.connections.len());
                        println!("connection accepted - track assignments:");
                        for (track, assignee) in state.track_assignments.iter() {
                            println!("\ttrack {} => connection {}", track, assignee);
                        }
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
            // println!("sleeping for {:?}", time_until_note);
            sleep(time_until_note);
        }

        match event {
            &MusicalEvent::PlayNote { track, channel, note, duration, velocity, .. } => {
                let note = Step(note as f32);
                let duration = clocks_to_duration(&timing, duration);
                let volume = velocity as f32 / 128.0;
                let end_time = now + duration;
                if end_time >= latest_note_end_time {
                    latest_note_end_time = end_time;
                }
                // println!("[{}] beep at {:?} for {:?}", channel, note.to_letter_octave(), duration);

                let mut play_note = true;

                if let &Some(ref channels) = &included_channels {
                    if channels.contains(&(channel as usize)) == false {
                        play_note = false;
                    }
                }

                if let &Some(ref channels) = &excluded_channels {
                    if channels.contains(&(channel as usize)) {
                        play_note = false;
                    }
                }

                if let &Some(ref tracks) = &included_tracks {
                    if tracks.contains(&track) == false {
                        play_note = false;
                    }
                }

                if let &Some(ref tracks) = &excluded_tracks {
                    if tracks.contains(&track) {
                        play_note = false;
                    }
                }

                if play_note {
                    let state = shared_state.lock()
                        .expect("failed to lock mutex to send note");
                    if let Some(assigned_connection) = state.track_assignments.get(&track) {
                        if let Some(client) = state.connections.iter().nth(*assigned_connection) {
                            client.send(Packet::PlayNote {
                                duration: duration_to_nanoseconds(duration),
                                frequency: note.to_hz().0,
                                volume,
                            }).expect("failed to send note packet");
                        }
                    }
                }
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

    let mut state = shared_state.lock()
        .expect("failed to lock mutex for terminate packet");

    println!("telling clients to terminate...");
    for client in state.connections.iter_mut() {
        client.send( Packet::TerminateAfter(
            terminate_delay
        )).expect("failed to serialize termination packet");
    }

    println!("ensuring clients get termination messages...");
    for client in state.connections.iter_mut() {
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

fn assign_tracks(tracks: &[usize], connection_count: usize) -> HashMap<usize, usize> {
    let mut result = HashMap::new();

    let mut index = 0;
    for track in tracks {
        result.insert(*track, index);
        index = (index + 1) % connection_count;
    }

    result
}

fn match_number_list(matches: &ArgMatches, name: &str, kind: &str) -> Option<Vec<usize>> {
    let result = matches.values_of(name)
        .map(
            |values| values.map(
                |track| track.parse()
                    .expect(&format!("invalid {} number", kind)))
        .collect::<Vec<usize>>());

    println!("{}: {:?}", name, result);

    result
}
