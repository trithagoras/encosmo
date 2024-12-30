use encosmo_shared::Packet;
use uuid::Uuid;


/// messages to send between actors, NOT packets to be sent to clients
#[derive(Clone, Debug)]
pub enum Message {
    SendPacket (Packet),    // packet to be sent to the client
    ReceivePacket (Packet), // packet that has been received from the client
    Tick,
    PlayerConnected (Uuid),
    PlayerDisconnected (Uuid)
}
