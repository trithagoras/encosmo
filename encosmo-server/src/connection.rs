use std::sync::Arc;

use anyhow::Result;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::tcp::{self, OwnedReadHalf}, spawn, sync::{broadcast, mpsc, Mutex}};
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
    self_tx: mpsc::Sender<Message>,
    outbox_tx: mpsc::Sender<Packet>,
    outbox_rx: Arc<Mutex<mpsc::Receiver<Packet>>>
}

impl Connection {
    pub fn new(id: Uuid, client_tx: tcp::OwnedWriteHalf, server_chan: (mpsc::Sender<Message>, mpsc::Receiver<Message>), broadcast_rx: broadcast::Receiver<Message>) -> Connection {
        let (server_tx, server_rx) = server_chan;
        let (self_tx, self_rx) = mpsc::channel(100);
        let (outbox_tx, outbox_rx) = mpsc::channel(100);
        let outbox_rx = Arc::new(Mutex::new(outbox_rx));

        // resubscribe since likely some messages have been added to the channel while accepting connection
        Connection {
            id,
            client_tx, server_rx, server_tx,
            broadcast_rx: broadcast_rx.resubscribe(),
            self_rx, self_tx,
            outbox_tx, outbox_rx
        }
    }

    pub async fn start(&mut self, client_rx: tcp::OwnedReadHalf) -> Result<()> {
        let server_tx = self.server_tx.clone();
        let id = self.id;

        let self_tx = self.self_tx.clone();
        
        // fire off read bytes loop
        let mut t = spawn(recv_packet_loop(client_rx, self_tx));

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

        // loop finished indicates connection closed
        server_tx.send(Message::PlayerDisconnected(id)).await?;
        Ok (())
    }

    async fn process_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Tick => self.tick().await?,
            Message::SendPacket(p) => self.outbox_tx.send(p).await?,
            Message::PlayerConnected(id) => {
                if id == self.id {
                    self.outbox_tx.send(Packet::Id(id)).await?;
                }
                else {
                    self.outbox_tx.send(Packet::PlayerConnected(id)).await?;
                }
            },
            Message::PlayerDisconnected(id) => self.outbox_tx.send(Packet::PlayerDisconnected(id)).await?,
            Message::ReceivePacket(p) => self.process_packet(p).await?
        }
        Ok (())
    }

    async fn tick(&mut self) -> Result<()> {
        // send packets to client
        {
            let mut lock = self.outbox_rx.lock().await;
            while let Ok (p) = lock.try_recv() {
                send_packet(&mut self.client_tx, p).await?;
            }
        }
        Ok (())
    }

    async fn process_packet(&mut self, p: Packet) -> Result<()> {
        match p {
            Packet::UpdateComponent(id, comp) => {
                if id != self.id {
                    log::warn!("Client {} attempted to update component {:?} that doesn't belong to them: {}", self.id, comp, id);
                } else {
                    // TODO: send this packet to the server for further processing. Remove placeholder below.
                    log::info!("Client {} has updated their component to {:?}", id, comp);
                }
            }
            _ => {}
        }
        Ok (())
    }
}

async fn recv_packet_loop(mut client_rx: OwnedReadHalf, self_tx: mpsc::Sender<Message>) -> Result<()> {
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
            Err (e) => log::error!("Packet deserialization failed from string {}. Error: {}", s, e),
            Ok (packet) => {
                self_tx.send(Message::ReceivePacket(packet)).await?;
            }
        }
    }
}

async fn send_packet(client_tx: &mut tcp::OwnedWriteHalf, packet: Packet) -> Result<()> {
    let s = serde_json::to_string(&packet)?;
    let bytes = s.as_bytes();
    client_tx.write(bytes).await?;
    Ok (())
}