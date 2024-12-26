use std::{collections::HashMap, sync::Arc};

use tokio::{io, net::TcpListener, spawn, sync::{broadcast, mpsc, Mutex}};      // todo: should add a broadcast channel as well
use uuid::Uuid;

use crate::{connection::Connection, messages::Message};

pub struct Server {
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    broadcast_tx: broadcast::Sender<Message>,
    server_tx: mpsc::Sender<Message>,
    server_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl Server {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let (connection_tx, connection_rx) = mpsc::channel(100);
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            server_tx: connection_tx,
            server_rx: Arc::new(Mutex::new(connection_rx))
        }
    }

    pub async fn start(&mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        log::info!("SERVER: listening on port {}", port);

        // fire off accept loop then block on receive message loop

        let broadcast_tx = self.broadcast_tx.clone();
        let server_tx = self.server_tx.clone();
        let connections = self.connections.clone();
        spawn(async move {
            loop {
                let broadcast_rx = broadcast_tx.subscribe();
                _ = Self::accept_connection(broadcast_rx, server_tx.clone(), connections.clone(), &listener).await;
            }
        });

        let server_rx = self.server_rx.clone();
        loop {
            let mut lock = server_rx.lock().await;
            let msg = lock.recv().await.unwrap();
            self.dispatch_msg(msg).await;
        }
    }

    async fn accept_connection(broadcast_rx: broadcast::Receiver<Message>, server_tx: mpsc::Sender<Message>, connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>, listener: &TcpListener) -> io::Result<()> {
        let (stream, _) = listener.accept().await?;
        let (client_rx, client_tx) = stream.into_split();
        let id = Uuid::new_v4();
        let (conn_tx, conn_rx) = mpsc::channel(100);
        let chan = (server_tx.clone(), conn_rx);

        let mut connection = Connection::new(Some(id), client_tx, chan, broadcast_rx);

        let mut lock = connections.lock().await;
        lock.insert(id, conn_tx);

        // clone
        let connections = connections.clone();

        spawn(async move {
            match connection.start(client_rx).await {
                Err (e) => log::error!("Client {} disconnected with error {}", id, e),
                _ => log::info!("Player {} has disconnected gracefully.", id)
            }
            // connection has finished
            connections.lock().await.remove(&id);
        });

        _ = server_tx.send(Message::Connected(id)).await;
        log::info!("New connection: {}", id);

        Ok (())
    }

    async fn dispatch_msg(&mut self, msg: Message) {
        match msg {
            Message::Connected(_) => {
                _ = self.broadcast_tx.send(msg);
            },
            Message::Disconnected(_) => {
                _ = self.broadcast_tx.send(msg);
            },
            _ => log::warn!("SERVER: Unhandled message received: {:?}", msg)
        }
    }
}