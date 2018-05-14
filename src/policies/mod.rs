
mod by_channel;
mod broadcast;
mod by_track;

use super::connection::{ClientUID, ClientInfo};
use self::by_channel::ByChannelPolicy;
use self::broadcast::BroadcastPolicy;
use self::by_track::ByTrackPolicy;
use super::midi::Note;

pub trait ClientSelectionPolicy: Send {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]);
    fn select_clients(&self, note: &Note) -> Vec<ClientUID>;
}

pub fn select_policy(name: String) -> Option<Box<ClientSelectionPolicy>> {
    match name.to_lowercase().as_str() {
        "broadcast" => Some(Box::new(BroadcastPolicy::new())),
        "by-track" => Some(Box::new(ByTrackPolicy::new())),
        "by-channel" => Some(Box::new(ByChannelPolicy::new())),
        _ => None,
    }
}
