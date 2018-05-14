use std::collections::HashMap;

use itertools::Itertools;

use super::super::connection::{ClientUID, ClientInfo};
use super::super::midi::{Note, MusicalEvent};
use super::ClientSelectionPolicy;

struct FrequencyRangeAssignment {
    lowest: u8,
    highest: u8,
    client: ClientUID,
}

pub struct ByFrequencyPolicy {
    note_histogram: HashMap<u8, usize>,
    assignments: Vec<FrequencyRangeAssignment>,
    spread: usize,
}

impl ByFrequencyPolicy {
    pub fn new(events: &[MusicalEvent], spread: usize) -> Self {
        Self {
            note_histogram: build_histogram(events),
            assignments: Vec::new(),
            spread,
        }
    }
}

fn build_histogram(events: &[MusicalEvent]) -> HashMap<u8, usize> {
    let mut result = HashMap::new();

    for event in events.iter() {
        match event {
            MusicalEvent::PlayNote(Note { note, .. }) => {
                result.entry(*note)
                    .and_modify(|entry| *entry += 1)
                    .or_insert(1);
            },
            _ => {},
        }
    }

    result
}

impl ClientSelectionPolicy for ByFrequencyPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        self.assignments = if clients.len() > 0 {
            let total_note_count: usize = self.note_histogram.values()
                .map(|v| *v)
                .sum();
            let ideal_notes_per_client = (total_note_count / clients.len()) / self.spread;

            let mut new_assignments = Vec::new();
            let mut next_client_index = 0;
            let mut assigned_count = 0;

            let histogram_sorted = self.note_histogram.iter()
                .sorted();

            for (note, count) in histogram_sorted {
                let start_new_assignment = new_assignments.is_empty() || assigned_count >= ideal_notes_per_client;

                if start_new_assignment {
                    let client_index = next_client_index % clients.len();
                    next_client_index += 1;

                    new_assignments.push(FrequencyRangeAssignment {
                        lowest: *note,
                        highest: *note,
                        client: clients[client_index].uid.clone(),
                    });
                    assigned_count = *count;
                } else {
                    let last_assignment = new_assignments.last_mut().unwrap();

                    last_assignment.highest = *note;
                    assigned_count += *count;
                }
            }

            if new_assignments.len() > 0 {
                println!("assignments:");
                for assignment in new_assignments.iter() {
                    println!(
                        "  notes [{} -> {}] => {:?}",
                        assignment.lowest,
                        assignment.highest,
                        assignment.client
                    );
                }
            }

            new_assignments
        } else {
            vec![]
        }
    }

    fn select_clients(&self, note: &Note) -> Vec<ClientUID> {
        self.assignments.iter()
            .filter(|assignment| {
                note.note >= assignment.lowest && note.note <= assignment.highest
            })
            .map(|assignment| assignment.client.clone())
            .collect::<Vec<_>>()
    }
}
