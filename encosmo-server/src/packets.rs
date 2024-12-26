use serde::Serialize;
use uuid::Uuid;


#[derive(Serialize, Clone, Debug)]
pub enum Packet {
    // client-server
    Login(String, String),
    Register(String, String),
    Logout,


    // server-client
    Id(Uuid)
}