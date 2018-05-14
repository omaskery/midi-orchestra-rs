use super::super::connection::{ClientUID, ClientInfo};
use super::ClientSelectionPolicy;
use super::super::midi::Note;

pub struct BroadcastPolicy {
    all: Vec<ClientUID>,
}

impl BroadcastPolicy {
    pub fn new() -> Self {
        Self {
            all: Vec::new(),
        }
    }
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
