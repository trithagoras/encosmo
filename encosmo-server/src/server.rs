use std::{collections::HashMap, sync::Arc, time::Duration};
use anyhow::Result;

use encosmo_shared::server_components::*;
use specs::prelude::*;
use tokio::{net::TcpListener, spawn, sync::{broadcast, mpsc, Mutex}, time::sleep};
use uuid::Uuid;

use crate::{connection::Connection, entities::create_player, messages::Message, systems::*};

pub struct Server {
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    tick_rate: u8,
    broadcast_tx: broadcast::Sender<Message>,
    server_tx: mpsc::Sender<Message>,
    server_rx: mpsc::Receiver<Message>,
}

impl Server {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            tick_rate: 1,
            broadcast_tx,
            server_tx,
            server_rx
        }
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        log::info!("SERVER: listening on port {}", port);
    
        // set up ECS
        let world = Arc::new(Mutex::new(World::new()));
        {
            let mut lock = world.lock().await;
            lock.register::<Position>();
            lock.register::<Translate>();
            lock.register::<PlayerDetails>();
            lock.register::<GameObjectDetails>();
        }

        let mut dispatcher = DispatcherBuilder::new()
            .with_thread_local(MoveSystem)
            .build();
    
        let broadcast_tx = self.broadcast_tx.clone();
        let server_tx = self.server_tx.clone();
        let connections = self.connections.clone();
        let world_cpy = world.clone();
    
        // fire off accept loop
        spawn(async move {
            loop {
                let broadcast_rx = broadcast_tx.subscribe();
                match accept_connection(broadcast_rx, server_tx.clone(), connections.clone(), &listener, world_cpy.clone()).await {
                    Err (e) => log::error!("Error accepting new connection: {}", e),
                    _ => {}
                }
            }
        });

        // tick loop
        let sleep_time = 1. / self.tick_rate as f64;
        loop {
            sleep(Duration::from_secs_f64(sleep_time)).await;
            self.tick(world.clone(), &mut dispatcher).await?;
        }
    }

    async fn tick(&mut self, world: Arc<Mutex<World>>, dispatcher: &mut Dispatcher<'_, '_>) -> Result<()> {
        log::debug!("tick");

        // check if there are any messages to process
        while let Ok (msg) = self.server_rx.try_recv() {
            self.process_message(msg).await?;
        }
    
        // run all our systems
        {
            let mut lock = world.lock().await;
            dispatcher.dispatch(&lock);
            lock.maintain();
        }
        
        // send all packets from each connection's outbox
        self.broadcast_tx.send(Message::Tick)?;
        Ok (())
    }

    async fn process_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::PlayerConnected(_) | Message::PlayerDisconnected(_) => {
                self.broadcast_tx.send(msg)?;
            },
            _ => {}
        }

        Ok (())
    }
}

async fn accept_connection(
    broadcast_rx: broadcast::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    listener: &TcpListener,
    world: Arc<Mutex<World>>
) -> Result<()> {
    let (stream, _) = listener.accept().await?;
    let (client_rx, client_tx) = stream.into_split();
    let id = Uuid::new_v4();
    let (conn_tx, conn_rx) = mpsc::channel(100);
    let chan = (server_tx.clone(), conn_rx);

    let mut connection = Connection::new(id, client_tx, chan, broadcast_rx);


    // limiting lifetime of each lock
    {
        let mut lock = connections.lock().await;
        lock.insert(id, conn_tx);
    }

    {
        // TODO: we are adding player to world here, but never deleting them.
        let mut lock = world.lock().await;
        create_player(&mut lock, id);
    }
    
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

    server_tx.send(Message::PlayerConnected(id)).await?;
    log::info!("New connection: {}", id);

    Ok (())
}
