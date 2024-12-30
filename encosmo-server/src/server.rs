use std::{collections::HashMap, sync::Arc, time::Duration};
use anyhow::Result;

use bimap::BiMap;
use encosmo_shared::{server_components::*, Packet};
use specs::{prelude::*, storage::AccessMut};
use tokio::{net::TcpListener, spawn, sync::{broadcast, mpsc, Mutex}, time::{sleep, Instant}};
use uuid::Uuid;

use crate::{connection::Connection, entities::create_player, messages::Message, resources::ServerTx, systems::*};

pub struct Server {
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    tick_rate: u8,
    broadcast_tx: broadcast::Sender<Message>,
    server_tx: mpsc::Sender<Message>,
    server_rx: mpsc::Receiver<Message>,
    world: Arc<Mutex<World>>,
    player_entities: Arc<Mutex<BiMap<Uuid, u32>>>,
    systems_tx: std::sync::mpsc::Sender<Message>,
    systems_rx: std::sync::mpsc::Receiver<Message>,
}

impl Server {
    pub fn new(tick_rate: u8) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let (server_tx, server_rx) = mpsc::channel(100);
        let (systems_tx, systems_rx) = std::sync::mpsc::channel();
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            tick_rate,
            broadcast_tx,
            server_tx,
            server_rx,
            world: Arc::new(Mutex::new(World::new())),
            player_entities: Arc::new(Mutex::new(BiMap::default())),
            systems_tx,
            systems_rx
        }
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        log::info!("SERVER: listening on port {}", port);
    
        // set up ECS
        {
            let mut lock = self.world.lock().await;
            lock.register::<Position>();
            lock.register::<Translate>();
            lock.register::<PlayerDetails>();
            lock.register::<GameObjectDetails>();
            lock.insert(ServerTx(self.systems_tx.clone()));
        }

        let mut dispatcher = DispatcherBuilder::new()
            .with_thread_local(MoveSystem)
            .build();
    
        let broadcast_tx = self.broadcast_tx.clone();
        let server_tx = self.server_tx.clone();
        let connections = self.connections.clone();
        let world_cpy = self.world.clone();
        let player_entities_cpy = self.player_entities.clone();
    
        // fire off accept loop
        spawn(async move {
            loop {
                let broadcast_rx = broadcast_tx.subscribe();
                match accept_connection(broadcast_rx, server_tx.clone(), connections.clone(), &listener, world_cpy.clone(), player_entities_cpy.clone()).await {
                    Err (e) => log::error!("Error accepting new connection: {}", e),
                    _ => {}
                }
            }
        });

        // tick loop
        let sleep_time = 1. / self.tick_rate as f64;

        loop {
            // restart timer
            let start_time = Instant::now();

            self.tick(&mut dispatcher).await?;

            // get elapsed time
            let elapsed_time = start_time.elapsed();
            let sleep_duration = Duration::from_secs_f64(sleep_time).saturating_sub(elapsed_time);

            if sleep_duration.is_zero() {
                log::warn!("Time taken to tick exceeded expected time. Time taken was {:?}", elapsed_time);
            } else {
                // sleep for remaining time
                sleep(sleep_duration).await;
            }
        }
    }

    async fn tick(&mut self, dispatcher: &mut Dispatcher<'_, '_>) -> Result<()> {
        log::debug!("tick");

        // TODO: these will not be in order!! due to sync vs async message queues
        // check if there are any messages to process
        while let Ok (msg) = self.server_rx.try_recv() {
            self.process_message(msg).await?;
        }

        // also check messages sent from systems bc they can't share the same queue :(
        while let Ok (msg) = self.systems_rx.try_recv() {
            self.process_message(msg).await?;
        }
    
        // run all our systems
        {
            let mut lock = self.world.lock().await;
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
            Message::Packet(Packet::UpdateComponent(eid, ref comp)) => {
                match comp {
                    ServerComponentKind::Translate(t) => {
                        self.update_component(eid, t).await?;
                    },
                    _ => log::warn!("Client {} attempted to update component they cannot: {:?}", eid, comp)
                }
            },
            Message::BroadcastPacket(p) => {
                self.broadcast_tx.send(Message::SendPacket(p))?;
            }
            _ => {}
        }

        Ok (())
    }

    async fn update_component<T>(&mut self, eid: u32, new_component: &T) -> Result<()> 
    where
        T: UpdatableComponent,
    {
        let entities = self.player_entities.lock().await;
        let res = entities.get_by_right(&eid);
        if res.is_none() {
            log::error!("Attempting to update entity that doesn't exist");
            return Ok(());
        }
        let entity = self.world.lock().await.entities().entity(eid);

        let world = self.world.lock().await;
        let mut storage = world.write_storage::<T>();

        if let Some(mut storage) = storage.get_mut(entity) {
            let existing_component = storage.access_mut();
            *existing_component = (new_component).clone();
            log::info!("Updated component for entity: {}", eid);
        } else {
            log::error!("Entity {:?} does not have the specified component!", entity);
        }

        Ok(())
    }
}

async fn accept_connection(
    broadcast_rx: broadcast::Receiver<Message>,
    server_tx: mpsc::Sender<Message>,
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Message>>>>,
    listener: &TcpListener,
    world: Arc<Mutex<World>>,
    player_entities: Arc<Mutex<BiMap<Uuid, u32>>>
) -> Result<()> {
    let (stream, _) = listener.accept().await?;
    let (client_rx, client_tx) = stream.into_split();
    let id = Uuid::new_v4();
    let (conn_tx, conn_rx) = mpsc::channel(100);
    let chan = (server_tx.clone(), conn_rx);

    // limiting lifetime of each lock
    {
        let mut lock = connections.lock().await;
        lock.insert(id, conn_tx);
    }

    let entity_id: u32;

    {
        // TODO: we are adding player to world here, but never deleting them.
        let mut lock = world.lock().await;
        let player_entity = create_player(&mut lock, id);
        entity_id = player_entity.id();
        {
            let mut lock = player_entities.lock().await;
            lock.insert(id, player_entity.id());
        }
    }
    
    // clone
    let _connections = connections.clone();

    let mut connection = Connection::new(id, entity_id, client_tx, chan, broadcast_rx);

    spawn(async move {
        match connection.start(client_rx).await {
            Err (e) => log::error!("Client {} disconnected with error {}", id, e),
            _ => log::info!("Player {} has disconnected gracefully.", id)
        }
        // connection has finished
        _connections.lock().await.remove(&id);
    });

    // TODO:
    // send the world as we know it up to this point

    server_tx.send(Message::PlayerConnected(id)).await?;
    server_tx.send(Message::BroadcastPacket(Packet::PlayerEntityId(id, entity_id))).await?;
    log::info!("New connection: {}", id);

    Ok (())
}
