use encosmo_shared::Packet;
use uuid::Uuid;


/// messages to send between actors, NOT packets to be sent to clients
#[derive(Clone, Debug)]
pub enum Message {
    Connected (Uuid),
    Disconnected (Uuid),
    PacketReceived (Uuid, Packet),
    SendPacket (Packet),    // immediately send this packet to the client
    SetName (Uuid, String),
    GetName,                // asking: "what is your name?"
    Name (Uuid, String),    // responding: "my name is ..."
    NameTaken (Uuid, String),
    Tick
}
