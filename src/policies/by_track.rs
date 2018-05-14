use std::collections::{HashSet, HashMap};

use super::super::connection::{ClientUID, ClientInfo};
use super::super::midi::{Note, MusicalEvent};
use super::ClientSelectionPolicy;

pub struct ByTrackPolicy {
    tracks: HashSet<usize>,
    assignments: HashMap<usize, ClientUID>,
}

impl ByTrackPolicy {
    pub fn new(events: &[MusicalEvent]) -> Self {
        let tracks = events.iter()
            .filter_map(|event| {
                match event {
                    MusicalEvent::PlayNote(Note { track, .. }) => {
                        Some(*track)
                    },
                    _ => None,
                }
            })
            .collect::<HashSet<_>>();

        Self {
            tracks,
            assignments: HashMap::new(),
        }
    }
}

impl ClientSelectionPolicy for ByTrackPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        let mut assignments = HashMap::new();
        for (index, track) in self.tracks.iter().enumerate() {
            assignments.insert(*track, clients[index % clients.len()].uid.clone());
        }
        self.assignments = assignments;

        if self.assignments.len() > 0 {
            println!("assignments:");
            for (channel, uid) in self.assignments.iter() {
                println!("  track {} => {:?}", channel, uid);
            }
        }
    }

    fn select_clients(&self, note: &Note) -> Vec<ClientUID> {
        match self.assignments.get(&note.track) {
            Some(uid) => vec![uid.clone()],
            _ => vec![],
        }
    }
}
