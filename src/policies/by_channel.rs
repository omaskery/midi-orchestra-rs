use std::collections::{HashSet, HashMap};

use super::super::connection::{ClientUID, ClientInfo};
use super::super::midi::{Note, MusicalEvent};
use super::ClientSelectionPolicy;

pub struct ByChannelPolicy {
    channels: HashSet<u8>,
    assignments: HashMap<u8, ClientUID>,
}

impl ByChannelPolicy {
    pub fn new(events: &[MusicalEvent]) -> Self {
        let channels = events.iter()
            .filter_map(|event| {
                match event {
                    MusicalEvent::PlayNote(Note { channel, .. }) => {
                        Some(*channel)
                    },
                    _ => None,
                }
            })
            .collect::<HashSet<_>>();

        Self {
            channels,
            assignments: HashMap::new(),
        }
    }
}

impl ClientSelectionPolicy for ByChannelPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        let mut assignments = HashMap::new();
        for (index, channel) in self.channels.iter().enumerate() {
            assignments.insert(*channel, clients[index % clients.len()].uid.clone());
        }
        self.assignments = assignments;

        if self.assignments.len() > 0 {
            println!("assignments:");
            for (channel, uid) in self.assignments.iter() {
                println!("  channel {} => {:?}", channel, uid);
            }
        }
    }

    fn select_clients(&self, note: &Note) -> Vec<ClientUID> {
        match self.assignments.get(&note.channel) {
            Some(uid) => vec![uid.clone()],
            _ => vec![],
        }
    }
}
