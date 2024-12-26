use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::tcp, spawn, sync::{broadcast, mpsc, watch}};
use uuid::Uuid;

use crate::{messages::Message, packets::Packet};

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

        // resubscribe since likely some messages have been added to the channel
        Connection { id, client_tx, server_rx, server_tx, broadcast_rx: broadcast_rx.resubscribe(), self_rx, self_tx }
    }

    pub async fn start(&mut self, mut client_rx: tcp::OwnedReadHalf) {
        // fire off read bytes loop
        let (tx, mut rx) = watch::channel(false);
        let server_tx = self.server_tx.clone();
        let id = self.id;

        let self_tx = self.self_tx.clone();
        
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
                let s = String::from_utf8(buf.to_vec()).unwrap();
                let res = serde_json::from_str::<Packet>(&s.as_str());
                match res {
                    Err (e) => log::error!("{}: packet deserialization failed from string {}. Error: {}", id, s, e),
                    Ok (packet) => {
                        _ = self_tx.send(Message::PacketReceived(packet)).await;
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
                    self.dispatch(msg).await;
                }
                msg = self.broadcast_rx.recv() => {
                    let msg = msg.unwrap();
                    self.dispatch(msg).await;
                }
                msg = self.self_rx.recv() => {
                    let msg = msg.unwrap();
                    self.dispatch(msg).await;
                }
                _ = rx.changed() => {
                    // socket closed
                    break;
                }
            }
        }
    }

    async fn dispatch(&mut self, msg: Message) {
        match msg {
            Message::Connected(id) => {},
            Message::Disconnected(id) => {},
            Message::PacketReceived(p) => {
                self.dispatch_packet(p).await;
            },
            _ => log::warn!("{}: unhandled message received: {:?}", self.id, msg)
        };
    }

    async fn dispatch_packet(&mut self, p: Packet) {
        match p {
            Packet::Login(username, password) => log::info!("{}: login requested with username and password: {} {}", self.id, username, password),
            _ => {}
        }
    }

    async fn send_packet(&mut self, packet: Packet) -> io::Result<()> {
        let s = serde_json::to_string(&packet)?;
        let bytes = s.as_bytes();
        self.client_tx.write(bytes).await?;
        Ok (())
    }

    // pub async fn receive_message(&mut self, msg: Message) {
    //     match msg {
    //         Message
    //     }
    // }
}