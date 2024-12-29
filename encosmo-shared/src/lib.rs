use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Packet {
    // client-server
    SetName (String),
    Logout,

    // server-client
    Id (Uuid),
    PlayerConnected (Uuid),
    PlayerDisconnected (Uuid),
    NameTaken (String),
    Name (Uuid, String)     // player (id) has set their name to (string)
}