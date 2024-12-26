use std::sync::Arc;

use tokio::{io, sync::{mpsc, Mutex}};

use crate::{connection::Connection, messages::Message, packets::Packet};

use super::entry::Entry;

pub trait StateHandler {
    async fn dispatch_msg(&self, self_tx: Arc<Mutex<mpsc::Sender<Message>>>, msg: Message) -> io::Result<()>;
    async fn dispatch_packet(&self, self_tx: Arc<Mutex<mpsc::Sender<Message>>>, packet: Packet) -> io::Result<()>;
}

pub enum State {
    Entry (Entry)
}