use super::super::connection::{ClientUID, ClientInfo};
use super::ClientSelectionPolicy;
use super::super::midi::Note;

pub struct ByTrackPolicy {
    all: Vec<ClientUID>,
}

impl ByTrackPolicy {
    pub fn new() -> Self {
        Self {
            all: Vec::new(),
        }
    }
}

impl ClientSelectionPolicy for ByTrackPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        self.all = clients.iter()
            .map(|c| c.uid.clone())
            .collect::<Vec<_>>();
    }

    fn select_clients(&self, note: &Note) -> Vec<ClientUID> {
        if self.all.len() == 0 {
            vec![]
        } else {
            let uid = self.all[note.track % self.all.len()];
            vec![uid]
        }
    }
}
