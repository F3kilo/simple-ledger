use std::net::{ToSocketAddrs, UdpSocket};

use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Transport {
    socket: UdpSocket,
}

impl Transport {
    pub fn new(addr: impl ToSocketAddrs) -> Option<Self> {
        let socket = UdpSocket::bind(addr).ok()?;
        Some(Self { socket })
    }

    pub fn send(&self, to: impl ToSocketAddrs, msg: &impl Serialize) -> Option<usize> {
        let string = serde_json::to_string(msg).ok()?;
        self.socket.send_to(&string.as_bytes(), to).ok()
    }

    pub fn receive<T: DeserializeOwned>(&self) -> Option<T> {
        let mut buf = [0; 1536];
        self.socket.recv(&mut buf).ok()?;

        let string = String::from_utf8(buf.to_vec()).ok()?;
        serde_json::from_str::<T>(&string).ok()
    }
}
