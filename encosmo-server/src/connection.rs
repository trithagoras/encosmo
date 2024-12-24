use serde::Serialize;
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::{tcp, TcpStream}, sync::{broadcast, mpsc}};
use uuid::Uuid;

use crate::{messages::Message, packets::Packet, utils::Channel};

pub struct Connection {
    id: Uuid,
    client_rx: tcp::OwnedReadHalf,
    client_tx: tcp::OwnedWriteHalf,
    server_rx: mpsc::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    broadcast_rx: broadcast::Receiver<Message>
}

impl Connection {
    pub fn new(id: Option<Uuid>, stream: TcpStream, chan: Channel, broadcast_rx: broadcast::Receiver<Message>) -> Connection {
        let id = match id {
            Some (id) => id,
            None => Uuid::new_v4()
        };
        let (client_rx, client_tx) = stream.into_split();
        let (server_tx, server_rx) = chan;

        Connection { id, client_rx, client_tx, server_rx, server_tx, broadcast_rx }
    }

    pub async fn start(&mut self) {
        println!("New connection: {}", self.id);
        loop {
            let mut buf = [0u8; 1024];
            let res = self.client_rx.read(&mut buf).await;
            if let Err (_) = res {
                eprintln!("error reading socket");
                continue;
            }
            let read = res.unwrap();
            if read == 0 {
                println!("socket closed gracefully");
                return;
            }
            println!("bytes received: {:?}", buf);
        }
    }

    pub async fn send_packet(&mut self, packet: Packet) -> io::Result<()> {
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