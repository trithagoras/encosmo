
use std::{error::Error, fmt::Display};

use uuid::Uuid;

use crate::packets::Packet;

#[derive(Debug)]
pub enum MessageError {
    NotFound
}

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


impl Error for MessageError {
    
}

// messages to send between actors, NOT packets to be sent to clients
#[derive(Clone)]
pub enum Message {
    Connected (Uuid),
    Disconnected (Uuid),
    PacketReceived (Packet)
}