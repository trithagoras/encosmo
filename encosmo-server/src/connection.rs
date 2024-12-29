use anyhow::Result;
use log::warn;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::tcp, spawn, sync::{broadcast, mpsc}};
use uuid::Uuid;
use encosmo_shared::Packet;
use crate::messages::Message;

pub struct Connection {
    id: Uuid,
    client_tx: tcp::OwnedWriteHalf,
    server_rx: mpsc::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    broadcast_rx: broadcast::Receiver<Message>,
    self_rx: mpsc::Receiver<Message>,
    self_tx: mpsc::Sender<Message>
}

impl Connection {
    pub fn new(id: Option<Uuid>, client_tx: tcp::OwnedWriteHalf, server_chan: (mpsc::Sender<Message>, mpsc::Receiver<Message>), broadcast_rx: broadcast::Receiver<Message>) -> Connection {
        let id = match id {
            Some (id) => id,
            None => Uuid::new_v4()
        };
        let (server_tx, server_rx) = server_chan;
        let (self_tx, self_rx) = mpsc::channel(100);

        // resubscribe since likely some messages have been added to the channel while accepting connection
        Connection {
            id,
            client_tx, server_rx, server_tx,
            broadcast_rx: broadcast_rx.resubscribe(),
            self_rx, self_tx
        }
    }

    pub async fn start(&mut self, mut client_rx: tcp::OwnedReadHalf) -> Result<()> {
        let server_tx = self.server_tx.clone();
        let id = self.id;

        let self_tx = self.self_tx.clone();
        
        // fire off read bytes loop
        let mut t: tokio::task::JoinHandle::<Result<()>> = spawn(async move {
            loop {
                let mut buf = [0u8; 1024];
                let read = client_rx.read(&mut buf).await?;
                if read == 0 {
                    return Ok (());
                }
                let res = String::from_utf8(buf[0..read].to_vec());
                if let Err (e) = res {
                    log::error!("Error reading buffer to utf8 bytes: {}", e);
                    continue;
                }
                let s = res.unwrap();
                match serde_json::from_str::<Packet>(&s.as_str()) {
                    Err (e) => log::error!("{}: packet deserialization failed from string {}. Error: {}", id, s, e),
                    Ok (packet) => {
                        self_tx.send(Message::PacketReceived(id, packet)).await?;
                    }
                }
            }
        });

        // block on message read loop
        loop {
            tokio::select! {
                msg = self.server_rx.recv() => {
                    let msg = msg.unwrap();
                    self.process_message(msg).await?;
                }
                msg = self.broadcast_rx.recv() => {
                    let msg = msg.unwrap();
                    self.process_message(msg).await?;
                }
                msg = self.self_rx.recv() => {
                    let msg = msg.unwrap();
                    self.process_message(msg).await?;
                }
                _ = &mut t => {
                    // socket closed
                    break;
                }
            }
        }

        server_tx.send(Message::Disconnected(id)).await?;
        Ok (())
    }

    async fn process_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            // global message handling
            Message::Connected(id) => {
                if id == self.id {
                    // our own connected message.
                    self.send_packet(Packet::Id(id)).await?;
                } else {
                    self.send_packet(Packet::PlayerConnected(id)).await?;
                }
            },
            Message::Disconnected(id) => {
                self.send_packet(Packet::PlayerDisconnected(id)).await?;
            },
            Message::PacketReceived(_, ref p) => {
                self.dispatch_packet(p.clone()).await?;
            },
            Message::Name(id, name) => {
                self.send_packet(Packet::Name(id, name)).await?;
            },
            Message::SendPacket(p) => {
                self.send_packet(p).await?;
            },
            _ => {}
        };
        
        Ok (())
    }

    async fn dispatch_packet(&mut self, p: Packet) -> Result<()> {
        match p {
            Packet::UpdateComponent(_id, comp) => {
                if _id != self.id {
                    log::warn!("Connection {} attempted to update a component not belonging to them {}", self.id, _id);
                    return Ok (())
                }
                log::info!("Updating component belonging to {}. Received: {:?}", self.id, comp);
            },
            _ => {}
        }
        Ok (())
    }

    async fn send_packet(&mut self, packet: Packet) -> Result<()> {
        let s = serde_json::to_string(&packet)?;
        let bytes = s.as_bytes();
        self.client_tx.write(bytes).await?;
        Ok (())
    }
}