
mod by_channel;
mod broadcast;
mod by_track;
mod by_freq;

use super::connection::{ClientUID, ClientInfo};
use super::midi::{MusicalEvent, Note};
use self::by_channel::ByChannelPolicy;
use self::by_freq::ByFrequencyPolicy;
use self::broadcast::BroadcastPolicy;
use self::by_track::ByTrackPolicy;

pub trait ClientSelectionPolicy: Send {
    fn on_clients_changed(&mut self, clients: &[ClientInfo]);
    fn select_clients(&self, note: &Note) -> Vec<ClientUID>;
}

pub fn select_policy(name: String, events: &[MusicalEvent]) -> Option<Box<ClientSelectionPolicy>> {
    match name.to_lowercase().as_str() {
        "broadcast" => Some(Box::new(BroadcastPolicy::new())),
        "by-track" => Some(Box::new(ByTrackPolicy::new())),
        "by-channel" => Some(Box::new(ByChannelPolicy::new(events))),
        "by-freq" => Some(Box::new(ByFrequencyPolicy::new(events))),
        _ => None,
    }
}
