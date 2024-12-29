use anyhow::Result;
use blueprints::create_player;
use components::*;
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

    // set up ECS
    let mut world = World::new();
    world.register::<Pos>();
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
        clear_background(BLACK);

        // Run systems
        dispatcher.dispatch(&mut world);
        world.maintain();

        draw_text("text", 32., 32., 32., YELLOW);

        set_default_camera();
        next_frame().await
    }
}
