use std::sync::Arc;

use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::tcp, spawn, sync::{broadcast, mpsc, watch, Mutex}};
use uuid::Uuid;

use crate::{messages::Message, packets::Packet, states::{entry::Entry, state::{State, StateHandler}}};

pub struct Connection {
    id: Uuid,
    client_tx: tcp::OwnedWriteHalf,
    server_rx: mpsc::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    broadcast_rx: broadcast::Receiver<Message>,
    self_rx: mpsc::Receiver<Message>,
    self_tx: Arc<Mutex<mpsc::Sender<Message>>>,
    state: State
}

impl Connection {
    pub fn new(id: Option<Uuid>, client_tx: tcp::OwnedWriteHalf, server_chan: (mpsc::Sender<Message>, mpsc::Receiver<Message>), broadcast_rx: broadcast::Receiver<Message>) -> Connection {
        let id = match id {
            Some (id) => id,
            None => Uuid::new_v4()
        };
        let (server_tx, server_rx) = server_chan;
        let (self_tx, self_rx) = mpsc::channel(100);
        let state = State::Entry(Entry { conn_id: id });

        // resubscribe since likely some messages have been added to the channel
        Connection {
            id, client_tx, server_rx, server_tx,
            broadcast_rx: broadcast_rx.resubscribe(),
            self_rx, self_tx: Arc::new(Mutex::new(self_tx)),
            state
        }
    }

    pub async fn start(&mut self, mut client_rx: tcp::OwnedReadHalf) -> io::Result<()> {
        // fire off read bytes loop
        let (tx, mut rx) = watch::channel(false);
        let server_tx = self.server_tx.clone();
        let id = self.id;

        let self_tx_rc = self.self_tx.clone();
        
        spawn(async move {
            loop {
                let mut buf = [0u8; 1024];
                let res = client_rx.read(&mut buf).await;
                if let Err (_) = res {
                    break;
                }
                let read = res.unwrap();
                if read == 0 {
                    break;
                }
                // TODO: error handling on from_utf8
                let s = String::from_utf8(buf[0..read].to_vec()).unwrap();
                let res = serde_json::from_str::<Packet>(&s.as_str());
                match res {
                    Err (e) => log::error!("{}: packet deserialization failed from string {}. Error: {}", id, s, e),
                    Ok (packet) => {
                        let lock = self_tx_rc.lock().await;
                        _ = lock.send(Message::PacketReceived(packet)).await;
                    }
                }
            }
            _ = tx.send(true);
            _ = server_tx.send(Message::Disconnected(id)).await;
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
                _ = rx.changed() => {
                    // socket closed
                    break;
                }
            }
        }

        Ok (())
    }

    async fn dispatch_msg(&mut self, msg: Message) -> io::Result<()> {
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
            Message::PacketReceived(ref p) => {
                self.dispatch_packet(p.clone()).await?;
            },
            _ => {
                // state-specific message handling
                let rc = self.self_tx.clone();
                match &self.state {
                    State::Entry (state) => state.dispatch_msg(rc, msg).await?,
                }
            }
        };
        
        Ok (())
    }

    async fn dispatch_packet(&mut self, p: Packet) -> io::Result<()> {
        let rc = self.self_tx.clone();
        match &self.state {
            State::Entry (state) => state.dispatch_packet(rc, p).await?,
        }

        Ok (())
    }

    async fn send_packet(&mut self, packet: Packet) -> io::Result<()> {
        let s = serde_json::to_string(&packet)?;
        let bytes = s.as_bytes();
        self.client_tx.write(bytes).await?;
        Ok (())
    }
}