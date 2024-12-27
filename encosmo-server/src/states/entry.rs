use std::sync::Arc;

use tokio::{io, sync::{mpsc, Mutex}};
use uuid::Uuid;

use crate::{messages::Message, packets::Packet};

use super::state::StateHandler;

pub struct Entry {
    pub conn_id: Uuid
}

impl StateHandler for Entry {
    async fn dispatch_msg(&self, self_tx: Arc<Mutex<mpsc::Sender<Message>>>, msg: Message) -> io::Result<()> {
        match msg {
            Message::PacketReceived(p) => self.dispatch_packet(self_tx, p).await?,
            _ => log::warn!("{}: unhandled message in ENTRY state: {:?}", self.conn_id, msg)
        };
        Ok (())
    }

    async fn dispatch_packet(&self, self_tx: Arc<Mutex<mpsc::Sender<Message>>>, packet: crate::packets::Packet) -> io::Result<()> {
        match packet {
            Packet::Login(username) => log::info!("{}: login requested with username: {}", self.conn_id, username),
            _ => {}
        };
        Ok (())
    }
}
