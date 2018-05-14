use super::super::connection::{ClientUID, ClientInfo};
use super::ClientSelectionPolicy;
use super::super::midi::Note;

pub struct ByChannelPolicy {
    all: Vec<ClientUID>,
}

impl ByChannelPolicy {
    pub fn new() -> Self {
        Self {
            all: Vec::new(),
        }
    }
}

impl ClientSelectionPolicy for ByChannelPolicy {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]) {
        self.all = clients.iter()
            .map(|c| c.uid.clone())
            .collect::<Vec<_>>();
    }

    fn select_clients(&self, note: &Note) -> Vec<ClientUID> {
        if self.all.len() == 0 {
            vec![]
        } else {
            let uid = self.all[(note.channel as usize) % self.all.len()];
            vec![uid]
        }
    }
}
