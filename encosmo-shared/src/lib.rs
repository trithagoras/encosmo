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
    PlayerEntityId (Uuid, u32),
    Name (Uuid, String),     // player (id) has set their name to (string)
    UpdateComponent (u32, ServerComponentKind),     // update component belonging to entity with id {id}
    UpsertEntity (u32, Vec<ServerComponentKind>),   // either creates new entity or updates existing entity
}
