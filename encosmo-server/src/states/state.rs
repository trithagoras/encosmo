
use anyhow::Result;
use crate::{messages::Message, packets::Packet};

use super::entry::Entry;

pub trait StateHandler {
    async fn dispatch_msg(&self, msg: Message) -> Result<()>;
    async fn dispatch_packet(&self, packet: Packet) -> Result<()>;
}

pub enum State {
    Entry (Entry),
}