
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Packet {
    ClientInfo,
    PlayNote {
        duration: u64,
        frequency: f32,
        volume: f32,
    },
    TerminateAfter(u64),
}

impl Packet {
    pub fn is_client_message(&self) -> bool {
        match self {
            &Packet::ClientInfo => true,
            _ => false,
        }
    }
}
