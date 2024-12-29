use std::{collections::HashMap, sync::Arc};
use anyhow::Result;

use bimap::BiMap;
use encosmo_shared::server_components::*;
use specs::prelude::*;
use tokio::{net::TcpListener, spawn, sync::{broadcast, mpsc, Mutex}};
use uuid::Uuid;

use crate::{connection::Connection, messages::Message, systems::*};

pub struct Server {
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    player_names: Arc<Mutex<BiMap<String, Uuid>>>,
    broadcast_tx: broadcast::Sender<Message>,
    server_tx: mpsc::Sender<Message>,
    server_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl Server {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            player_names: Arc::new(Mutex::new(BiMap::new())),
            broadcast_tx,
            server_tx,
            server_rx: Arc::new(Mutex::new(server_rx)),
        }
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        log::info!("SERVER: listening on port {}", port);

        // set up ECS
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Translate>();

        // create_player(&mut world, &game_texture);

        let mut dispatcher = DispatcherBuilder::new()
            .with_thread_local(MoveSystem)
            .build();

        let broadcast_tx = self.broadcast_tx.clone();
        let server_tx = self.server_tx.clone();
        let connections = self.connections.clone();
        let player_names = self.player_names.clone();

        // fire off accept loop
        spawn(async move {
            loop {
                let broadcast_rx = broadcast_tx.subscribe();
                match Self::accept_connection(broadcast_rx, server_tx.clone(), connections.clone(), &listener, player_names.clone()).await {
                    Err (e) => log::error!("Error accepting new connection: {}", e),
                    _ => {}
                }
            }
        });

        let server_rx = self.server_rx.clone();
        loop {
            let mut lock = server_rx.lock().await;
            let msg = lock.recv().await.unwrap();
            self.dispatch_msg(msg).await?;
        }
    }

    async fn accept_connection(
        broadcast_rx: broadcast::Receiver<Message>,
        server_tx: mpsc::Sender<Message>,
        connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
        listener: &TcpListener,
        player_names: Arc<Mutex<BiMap<String, Uuid>>>
    ) -> Result<()> {
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
        let player_names = player_names.clone();

        spawn(async move {
            match connection.start(client_rx).await {
                Err (e) => log::error!("Client {} disconnected with error {}", id, e),
                _ => log::info!("Player {} has disconnected gracefully.", id)
            }
            // connection has finished
            connections.lock().await.remove(&id);
            player_names.lock().await.remove_by_right(&id);
        });

        server_tx.send(Message::Connected(id)).await?;
        log::info!("New connection: {}", id);

        Ok (())
    }

    async fn dispatch_msg(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Connected(_) => {
                self.broadcast_tx.send(msg)?;
            },
            Message::Disconnected(_) => {
                self.broadcast_tx.send(msg)?;
            },
            Message::SetName(id, name) => {
                self.set_player_name(id, name).await?;
            },
            _ => log::warn!("SERVER: Unhandled message received: {:?}", msg)
        };
        Ok (())
    }

    async fn set_player_name(&mut self, id: Uuid, name: String) -> Result<()> {
        log::info!("Attempting to set name '{}' for connection {}", name, id);
        let mut names = self.player_names.lock().await;
        if names.contains_left(&name) {
            // name already taken
            log::warn!("Name '{}' is already taken!", name);
            if let Some(conn_tx) = self.connections.lock().await.get(&id) {
                conn_tx.send(Message::NameTaken(id, name)).await?;
            }
        } else {
            names.insert(name.clone(), id);
            log::info!("Name '{}' set for connection {}", name, id);
            // Notify all clients of the new name assignment
            self.broadcast_tx.send(Message::Name(id, name))?;
        }
        Ok (())
    }

    
}