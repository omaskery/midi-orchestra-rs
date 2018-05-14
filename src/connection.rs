use bincode::{serialize_into, deserialize_from};
use bincode;

use std::net::TcpStream;
use std::mem::replace;

use super::packet::Packet;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ClientUID(usize);

impl ClientUID {
    fn new(uid: usize) -> Self {
        ClientUID(uid)
    }
}

pub struct ClientUIDFactory {
    next: ClientUID,
}

impl ClientUIDFactory {
    pub fn new() -> Self {
        Self {
            next: ClientUID::new(1),
        }
    }

    pub fn make(&mut self) -> ClientUID {
        let next = ClientUID::new(self.next.0 + 1);
        replace(&mut self.next, next)
    }
}

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub uid: ClientUID,
}

impl ClientInfo {
    pub fn new(uid: ClientUID) -> Self {
        Self {
            uid,
        }
    }
}

pub struct Connection {
    pub info: ClientInfo,
    pub stream: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream, info: ClientInfo) -> Self {
        stream.set_nodelay(true)
            .expect("failed to set connection to be no-delay");

        Self {
            stream,
            info,
        }
    }
}

impl Connection {
    pub fn send(&self, packet: Packet) -> Result<(), Box<bincode::ErrorKind>> {
        serialize_into(&self.stream, &packet)
    }

    pub fn recv(&self) -> Result<Packet, Box<bincode::ErrorKind>> {
        deserialize_from(&self.stream)
    }
}
