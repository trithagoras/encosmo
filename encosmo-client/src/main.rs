use std::{io::{Read, Write}, net::TcpStream, sync::mpsc, thread::spawn};

use anyhow::Result;
use blueprints::create_player;
use components::*;
use encosmo_shared::{server_components::{Position, Translate}, Packet};
use macroquad::prelude::*;
use specs::{DispatcherBuilder, World, WorldExt};
use systems::*;

mod blueprints;
mod components;
mod systems;
mod constants;


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
    let mut stream_cpy = stream.try_clone()?;

    // set up connection to server
    let (tx, rx) = mpsc::channel::<Packet>();
    let handle = spawn(move || -> Result<()> {
        loop {
            let mut buf = [0u8; 1024];
            let read = stream_cpy.read(&mut buf)?;
    
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
    });
    

    // set up ECS
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Translate>();
    world.register::<PlayerInput>();
    world.register::<Render>();
    world.register::<FollowCamera>();

    create_player(&mut world, &game_texture);

    // with_thread_local means the systems are run sequentually, so order matters
    let mut dispatcher = DispatcherBuilder::new()
        .with_thread_local(InputSystem)
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
        while let Ok (packet) = rx.try_recv() {
            dispatch_packet(packet.clone())?;
        }

        // Run systems
        dispatcher.dispatch(&mut world);
        world.maintain();

        draw_text("text", 32., 32., 32., YELLOW);

        set_default_camera();
        next_frame().await
    }
}

fn dispatch_packet(p: Packet) -> Result<()> {
    match p {
        Packet::Id(id) => println!("Your ID has been set to {}", id),
        Packet::PlayerConnected(id) => println!("A new player has connected: {}", id),
        Packet::PlayerDisconnected(id) => println!("Player has disconnected: {}", id),
        Packet::Name(id, name) => println!("Player with id {} has set their name to {}", id, name),
        p => println!("Received unhandled packet: {:?}", p)
    }
    Ok (())
}