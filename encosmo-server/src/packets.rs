use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Packet {
    // client-server
    Login(String, String),
    Register(String, String),
    Logout,

    // server-client
    Id(Uuid),
    PlayerConnected(Uuid),
    PlayerDisconnected(Uuid)
}