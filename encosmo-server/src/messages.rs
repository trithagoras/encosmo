use uuid::Uuid;

use crate::packets::Packet;


// messages to send between actors, NOT packets to be sent to clients
#[derive(Clone, Debug)]
pub enum Message {
    Connected (Uuid),
    Disconnected (Uuid),
    PacketReceived (Packet),
}