use std::{io::{Read, Write}, net::TcpStream, sync::mpsc, thread::spawn};

use anyhow::Result;
use entities::create_player;
use components::*;
use encosmo_shared::{server_components::{Position, Translate}, Packet};
use macroquad::prelude::*;
use resources::ConnectionId;
use specs::{DispatcherBuilder, World, WorldExt};
use systems::*;

mod entities;
mod components;
mod systems;
mod constants;
mod resources;


fn window_conf() -> Conf {
    Conf {
        window_title: "Encosmo".to_owned(),
        window_width: 960,
        window_height: 800,
        icon: Some (constants::icon()),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<()> {

    // load content
    let game_texture = load_texture("content/art/game-tiles.png").await?;
    game_texture.set_filter(FilterMode::Nearest);

    let mut stream = TcpStream::connect(("127.0.0.1", 42523))?;
    let stream_cpy = stream.try_clone()?;

    // internal packet queue (enqueues from systems)
    let (packet_tx, packet_rx) = mpsc::channel::<Packet>();

    // set up connection to server
    let (server_tx, server_rx) = mpsc::channel::<Packet>();
    let handle = spawn(move || recv_packet_loop(stream_cpy, server_tx));

    // set up ECS
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Translate>();
    world.register::<PlayerInput>();
    world.register::<Render>();
    world.register::<FollowCamera>();
    world.register::<ServerEntityId>();

    // adding resources
    world.insert(ConnectionId::default());

    // with_thread_local means the systems are run sequentually, so order matters
    let mut dispatcher = DispatcherBuilder::new()
        .with_thread_local(InputSystem {
            packet_tx: packet_tx.clone()
        })
        .with_thread_local(MoveSystem)
        .with_thread_local(FollowCameraSystem)  // e.g. have camera follow run AFTER move system for late-update
        .with_thread_local(RenderSystem)
        .build();


    loop {
        if handle.is_finished() {
            let res = handle.join();
            match res {
                Err (e) => eprintln!("Connection to server has been severed due to error: {:?}", e),
                _ => eprintln!("Connection to server has been severed")
            }
            return Ok (());
        }

        clear_background(BLACK);

        // read packets
        while let Ok (packet) = server_rx.try_recv() {
            process_packet(packet.clone(), &mut world, &game_texture)?;
        }

        // Run systems
        dispatcher.dispatch(&mut world);
        world.maintain();

        // send any outgoing packets
        while let Ok (packet) = packet_rx.try_recv() {
            send_packet(&mut stream, packet)?;
        }

        draw_text("text", 32., 32., 32., YELLOW);

        set_default_camera();
        next_frame().await
    }
}

fn recv_packet_loop(mut stream: TcpStream, tx: mpsc::Sender<Packet>) -> Result<()> {
    loop {
        let mut buf = [0u8; 1024];
        let read = stream.read(&mut buf)?;
        if read == 0 {
            // connection closed
            return Ok (());
        }

        let res = String::from_utf8(buf[0..read].into());
        if let Err (e) = res {
            eprintln!("Error decoding buffer into string: {}", e);
            continue;
        }
        let str = res.unwrap();
        let res = serde_json::from_str::<Packet>(&str);
        if let Err (e) = res {
            eprintln!("Error converting string into packet: {}", e);
            continue;
        }
        let packet = res.unwrap();
        tx.send(packet)?;
    }
}

fn process_packet(p: Packet, world: &mut World, game_texture: &Texture2D) -> Result<()> {
    match p {
        Packet::Id(_id) => {
            let mut connection_id = world.write_resource::<ConnectionId>();
            connection_id.0 = _id;
            println!("Your ID has been set to {}", _id);
        },
        Packet::PlayerConnected(id) => println!("A new player has connected: {}", id),
        Packet::PlayerDisconnected(id) => println!("Player has disconnected: {}", id),
        Packet::Name(id, name) => println!("Player with id {} has set their name to {}", id, name),
        Packet::UpdateComponent(eid, kind) => {
            println!("Updating component {:?} for entity with id: {}", kind, eid);
        },
        Packet::PlayerEntityId(id, eid) => {
            let my_id = world.read_resource::<ConnectionId>().0;
            if id == my_id {
                create_player(world, game_texture, eid);
            }
        }
        p => println!("Received unhandled packet: {:?}", p)
    }
    Ok (())
}

fn send_packet(stream: &mut TcpStream, p: Packet) -> Result<()> {
    let str = serde_json::to_string(&p)?;
    let bs = str.as_bytes();
    stream.write(&bs)?;
    Ok (())
}