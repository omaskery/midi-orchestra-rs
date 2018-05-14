
mod broadcast;
mod by_track;

use super::connection::{ClientUID, ClientInfo};
use self::broadcast::BroadcastPolicy;
use super::midi::Note;

pub trait ClientSelectionPolicy: Send {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]);
    fn select_clients(&self, note: &Note) -> Vec<ClientUID>;
}

pub fn select_policy(name: String) -> Option<Box<ClientSelectionPolicy>> {
    match name.to_lowercase().as_str() {
        "broadcast" => Some(Box::new(BroadcastPolicy::new())),
        _ => None,
    }
}
