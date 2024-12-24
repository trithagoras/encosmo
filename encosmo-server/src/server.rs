use std::{collections::HashMap, sync::Arc};

use tokio::{io, net::TcpListener, spawn, sync::{mpsc, broadcast, Mutex}};      // todo: should add a broadcast channel as well
use uuid::Uuid;

use crate::{connection::Connection, messages::Message, utils::Channel};

pub struct Server {
    connections: Arc<Mutex<HashMap<Uuid, Channel>>>,
    broadcast_tx: broadcast::Sender<Message>
}

impl Server {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx
        }
    }

    pub async fn start(&mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        println!("Server listening on port {}", port);

        loop {
            let res = listener.accept().await;
            if let Err (e) = res {
                eprintln!("Error accepting new connection. {}", e);
                continue;
            }
            let (stream, _) = listener.accept().await?;
            let id = Uuid::new_v4();
            let (server_tx, server_rx) = mpsc::channel(100);
            let (conn_tx, conn_rx) = mpsc::channel(100);
            let chan = (server_tx, conn_rx);
            let mut connection = Connection::new(Some(id), stream, chan, self.broadcast_tx.subscribe());

            let mut lock = self.connections.lock().await;
            lock.insert(id, (conn_tx, server_rx));

            // clone
            let connections = self.connections.clone();
            let broadcast_tx = self.broadcast_tx.clone();

            spawn(async move {
                connection.start().await;
                // connection has finished
                // let everyone else know there's a disconnection
                connections.lock().await.remove(&id);
                broadcast_tx.send(Message::Disconnected(id));
            });

            self.broadcast_tx.send(Message::Connected(id));
        }
    }
}