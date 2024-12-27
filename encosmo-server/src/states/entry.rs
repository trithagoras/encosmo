use std::sync::Arc;
use anyhow::Result;

use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::{messages::Message, packets::Packet};

use super::state::StateHandler;

pub struct Entry {
    pub conn_id: Uuid,
    pub self_tx: Arc<Mutex<mpsc::Sender<Message>>>
}

impl StateHandler for Entry {
    async fn dispatch_msg(&self, msg: Message) -> Result<()> {
        match msg {
            Message::PacketReceived(p) => self.dispatch_packet(p).await?,
            _ => log::warn!("{}: unhandled message in ENTRY state: {:?}", self.conn_id, msg)
        };
        Ok (())
    }

    async fn dispatch_packet(&self, packet: crate::packets::Packet) -> Result<()> {
        match packet {
            Packet::SetName(username) => self.try_setname(&username).await,
            _ => Ok (())
        }
    }
}

impl Entry {
    async fn try_setname(&self, name: &str) -> Result<()> {
        log::info!("{}: login requested with username: {}", self.conn_id, name);
        Ok (())
    }
}
