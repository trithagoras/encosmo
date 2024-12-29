use serde::{Deserialize, Serialize};
use server_components::ServerComponentKind;
use uuid::Uuid;

pub mod server_components;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
    // client-server
    SetName (String),
    Logout,

    // server-client
    Id (Uuid),
    PlayerConnected (Uuid),
    PlayerDisconnected (Uuid),
    Name (Uuid, String),     // player (id) has set their name to (string)
    // TODO: make a packet to update many components simultaneously. e.g. [(UUID, ServerComponentKind)]
    UpdateComponent (Uuid, ServerComponentKind),     // update component belonging to entity with id {id}
}
