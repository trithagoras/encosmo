use std::sync::Arc;

use anyhow::Result;

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::tcp, spawn, sync::{broadcast, mpsc, Mutex}};
use uuid::Uuid;

use crate::{details::Details, messages::Message, packets::Packet, states::{entry::Entry, state::{State, StateHandler}}};

pub struct Connection {
    client_tx: tcp::OwnedWriteHalf,
    server_rx: mpsc::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    broadcast_rx: broadcast::Receiver<Message>,
    self_rx: mpsc::Receiver<Message>,
    self_tx: mpsc::Sender<Message>,
    state: State,
    details: Arc<Mutex<Details>>
}

impl Connection {
    pub fn new(id: Option<Uuid>, client_tx: tcp::OwnedWriteHalf, server_chan: (mpsc::Sender<Message>, mpsc::Receiver<Message>), broadcast_rx: broadcast::Receiver<Message>) -> Connection {
        let id = match id {
            Some (id) => id,
            None => Uuid::new_v4()
        };
        let (server_tx, server_rx) = server_chan;
        let (self_tx, self_rx) = mpsc::channel(100);

        let details = Arc::new(Mutex::new(Details { id, name: None }));
        let state = State::Entry(Entry { details: details.clone(), self_tx: self_tx.clone(), server_tx: server_tx.clone() });


        // resubscribe since likely some messages have been added to the channel while accepting connection
        Connection {
            client_tx, server_rx, server_tx,
            broadcast_rx: broadcast_rx.resubscribe(),
            self_rx, self_tx,
            state, details
        }
    }

    pub async fn start(&mut self, mut client_rx: tcp::OwnedReadHalf) -> Result<()> {
        let server_tx = self.server_tx.clone();
        let id = self.details.lock().await.id;

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
                    self.dispatch_msg(msg).await?;
                }
                msg = self.broadcast_rx.recv() => {
                    let msg = msg.unwrap();
                    self.dispatch_msg(msg).await?;
                }
                msg = self.self_rx.recv() => {
                    let msg = msg.unwrap();
                    self.dispatch_msg(msg).await?;
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

    async fn dispatch_msg(&mut self, msg: Message) -> Result<()> {
        match msg {
            // global message handling
            Message::Connected(id) => {
                if id == self.details.lock().await.id {
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
            }
            _ => {
                // state-specific message handling
                match &mut self.state {
                    State::Entry (state) => state.dispatch_msg(msg).await?,
                }
            }
        };
        
        Ok (())
    }

    async fn dispatch_packet(&mut self, p: Packet) -> Result<()> {
        match &mut self.state {
            State::Entry (state) => state.dispatch_packet(p).await?,
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