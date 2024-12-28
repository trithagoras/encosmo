use std::sync::Arc;
use anyhow::Result;
use tokio::sync::{mpsc, Mutex};

use crate::{details::Details, messages::Message, packets::Packet};

use super::state::StateHandler;

pub struct Entry {
    pub details: Arc<Mutex<Details>>,
    pub self_tx: mpsc::Sender<Message>,
    pub server_tx: mpsc::Sender<Message>
}

impl StateHandler for Entry {
    async fn dispatch_msg(&self, msg: Message) -> Result<()> {
        match msg {
            Message::PacketReceived(_, p) => self.dispatch_packet(p).await?,
            Message::NameTaken(_, name) => self.self_tx.send(Message::SendPacket(Packet::NameTaken(name))).await?,
            Message::SetName(_, name) => {
                let mut lock = self.details.lock().await;
                lock.name = Some (name)
            },
            _ => log::warn!("{}: unhandled message in ENTRY state: {:?}", self.details.lock().await.id, msg)
        };
        Ok (())
    }

    async fn dispatch_packet(&self, packet: Packet) -> Result<()> {
        match packet {
            Packet::SetName(username) => self.try_setname(&username).await,
            _ => Ok (())
        }
    }
}

impl Entry {
    async fn try_setname(&self, name: &str) -> Result<()> {
        let lock = self.details.lock().await;
        log::info!("{}: has requested to set their name to {}", lock.id, name);
        self.server_tx.send(Message::SetName(lock.id, name.to_owned())).await?;
        Ok (())
    }
}
