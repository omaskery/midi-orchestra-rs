use midi::{MusicalEvent, Note, TimingChange};
use convert_duration::*;
use packet::Packet;
use midi;

use std::collections::{HashMap, HashSet};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::thread::{sleep, spawn};
use std::io::{Stdout, Write};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::hash::Hash;
use std::fmt::Debug;
use std;

use bincode::{serialize_into, deserialize_from};
use pbr::ProgressBar;
use pitch_calc::Step;
use clap::ArgMatches;
use term_size;
use bincode;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ClientUID(usize);

trait ClientSelectionPolicy: Send {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]);
    fn select_clients(&self, note: &Note) -> Vec<ClientUID>;
}

struct BroadcastPolicy {
    all: Vec<ClientUID>,
}

impl ClientSelectionPolicy for BroadcastPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        self.all = clients.iter()
            .map(|c| c.uid)
            .collect();
    }
    fn select_clients(&self, _note: &Note) -> Vec<ClientUID> {
        self.all.clone()
    }
}

fn select_policy(name: String) -> Option<Box<ClientSelectionPolicy>> {
    match name.to_lowercase().as_str() {
        "broadcast" => Some(Box::new(BroadcastPolicy {
            all: Vec::new()
        })),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct ClientInfo {
    uid: ClientUID,
}

struct Connection {
    info: ClientInfo,
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
    progress_bar: ProgressBar<Stdout>,
    width: usize,
    policy: Box<ClientSelectionPolicy>,
}

impl SharedState {
    fn new(music_length: u64, policy: Box<ClientSelectionPolicy>) -> Self {
        let width = match term_size::dimensions() {
            Some((w, _)) => w,
            _ => 80,
        };

        let mut progress_bar = ProgressBar::new(music_length);
        progress_bar.format("╢▌▌░╟");

        Self {
            connections: Vec::new(),
            progress_bar,
            width,
            policy,
        }
    }

    fn print_before(&self, text: &str) {
        println!("\r{}\r{}", std::iter::repeat(" ").take(self.width).collect::<String>(), text);
    }
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
    let verbose = matches.is_present("verbose");
    let volume_coefficient: f32 = match matches.value_of("volume").unwrap().parse() {
        Ok(value) => value,
        Err(_) => {
            println!("invalid volume value, must be floating point number");
            return;
        },
    };

    if volume_coefficient < 0.0 || volume_coefficient > 1.0 {
        println!("invalid volume value, must be between 0.0 and 1.0");
        return;
    }

    let included_tracks = number_list_to_hashset::<usize>(matches, "include track", "track");
    let excluded_tracks = number_list_to_hashset::<usize>(matches, "exclude track", "track");
    let included_channels = number_list_to_hashset::<u8>(matches, "include channel", "channel");
    let excluded_channels = number_list_to_hashset::<u8>(matches, "exclude channel", "channel");

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .expect("unable to create TCP server");

    let policy = select_policy("broadcast".to_string()).expect("invalid policy");

    println!("loading midi...");
    let music = midi::load_midi(path, verbose);

    let tracks = music.events().iter()
        .filter_map(|e| {
            if let &MusicalEvent::PlayNote(Note { track, .. }) = e {
                Some(track)
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();

    let channels = music.events().iter()
        .filter_map(|e| {
            if let &MusicalEvent::PlayNote(Note { channel, .. }) = e {
                Some(channel)
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();

    let tracks_before_filtering = tracks.clone();
    let channels_before_filtering = channels.clone();

    let mut excluded_channels = excluded_channels;
    if channels.contains(&10) && matches.is_present("allow channel 10") == false {
        println!("automatically ignoring channel 10");
        println!("  (set --allow-channel-10 to inhibit this)");

        excluded_channels.insert(10);
    }
    let excluded_channels = excluded_channels;

    let channels = channels.difference(&excluded_channels)
        .map(|v| *v)
        .collect::<HashSet<u8>>();
    let channels = channels.union(&included_channels)
        .map(|v| *v)
        .collect::<HashSet<u8>>();

    let tracks = tracks.difference(&excluded_tracks)
        .map(|v| *v)
        .collect::<HashSet<usize>>();
    let tracks = tracks.union(&included_tracks)
        .map(|v| *v)
        .collect::<HashSet<usize>>();

    println!("tracks:");
    println!("  {:?}", tracks_before_filtering);
    println!("  {:?}", tracks);
    println!("channels:");
    println!("  {:?}", channels_before_filtering);
    println!("  {:?}", channels);

    let events_to_play = music.events().iter()
        .filter(|e| {
            match e {
                MusicalEvent::PlayNote(Note { track, channel, .. }) => {
                    tracks.contains(track) && channels.contains(channel)
                },
                _ => true,
            }
        })
        .collect::<Vec<_>>();

    let shared_state_original = Arc::new(Mutex::new(
        SharedState::new(music.events().len() as u64, policy)
    ));

    let shared_state = shared_state_original.clone();
    spawn(move || {
        println!("accepting client connections...");
        let mut next_id = ClientUID(1);
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let mut state = shared_state_original.lock()
                        .expect("failed to acquire mutex while accepting");
                    let mut connection = Connection {
                        info: ClientInfo {
                            uid: next_id,
                        },
                        stream: s,
                    };
                    next_id = ClientUID(next_id.0 + 1);

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
                        state.print_before("connection accepted");
                        let clients_info = state.connections.iter()
                            .map(|c| c.info.clone())
                            .collect::<Vec<_>>();
                        state.policy.on_clients_changed(&clients_info);
                    } else {
                        state.print_before("connection rejected");
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
    println!("waiting {} seconds for clients to connect...", duration_to_seconds(delay_period));
    sleep(delay_period);

    println!("starting playback!");
    let mut latest_note_end_time = Instant::now();
    let start_time = Instant::now();
    for event in events_to_play.iter() {
        let start_offset = match event {
            MusicalEvent::PlayNote(Note { start_offset, .. }) => start_offset,
            MusicalEvent::TimingChange(TimingChange { start_offset, .. }) => start_offset,
        };

        let now = Instant::now();
        let event_time = start_time + *start_offset;

        if now < event_time {
            let time_until_note = event_time - now;
            {
                let mut state = shared_state.lock()
                    .expect("failed to acquire mutex to show sleep time");
                state.progress_bar.message(&format!("sleep: {:04}ms: ", (duration_to_seconds(time_until_note) * 1000f64) as u64));
            }
            sleep(time_until_note);
        }

        match event {
            MusicalEvent::PlayNote(note) => {
                let midi_note = Step(note.note as f32);
                let volume = (note.velocity as f32 / 128.0) * volume_coefficient;
                let end_time = now + note.duration;
                if end_time >= latest_note_end_time {
                    latest_note_end_time = end_time;
                }

                let state = shared_state.lock()
                    .expect("failed to lock mutex to send note");

                let assigned_connections = state.policy.select_clients(&note);

                for connection in state.connections.iter() {
                    if assigned_connections.contains(&connection.info.uid) {
                        connection.send(Packet::PlayNote {
                            duration: duration_to_nanoseconds(note.duration),
                            frequency: midi_note.to_hz().0,
                            volume,
                        }).expect("failed to send note packet");
                    }
                }
            },

            MusicalEvent::TimingChange(_timing_change) => {
                // we could emit timing information here, but we won't :)
            },
        }

        {
            let mut state = shared_state.lock()
                .expect("failed to acquire mutex to update progress bar");
            state.progress_bar.inc();
        }
    }

    let mut state = shared_state.lock()
        .expect("failed to lock mutex for shutdown processes");

    state.progress_bar.finish_println("playback complete\n");

    let now = Instant::now();
    let terminate_delay = if now < latest_note_end_time {
        duration_to_nanoseconds(latest_note_end_time - now)
    } else {
        0
    };

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

fn assign_tracks(tracks: &HashSet<usize>, connection_count: usize) -> HashMap<usize, usize> {
    let mut result = HashMap::new();

    let mut index = 0;
    for track in tracks {
        result.insert(*track, index);
        index = (index + 1) % connection_count;
    }

    result
}

fn number_list_to_hashset<T>(matches: &ArgMatches, name: &str, kind: &str) -> HashSet<T>
    where T: Eq + Debug + Hash + FromStr,
    <T as FromStr>::Err: Debug {
    let result = matches.values_of(name)
        .map(|values| {
            values.map(|track| {
                track.parse().expect(&format!("invalid {} number", kind))
            })
                .collect::<HashSet<T>>()
        })
        .unwrap_or_else(|| HashSet::new());

    println!("{}: {:?}", name, result);

    result
}
