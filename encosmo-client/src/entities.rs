use crate::components::*;
use encosmo_shared::server_components::{Position, Translate};
use macroquad::prelude::*;
use specs::{Builder, Entity, World, WorldExt};

pub fn create_player(world: &mut World, game_texture: &Texture2D, eid: u32) -> Entity {
    world
        .create_entity()
        .with(Translate::default())
        .with(Position::default())
        .with(PlayerInput)
        .with(Render {
            texture: game_texture.clone(),
            source: Rect::new(432., 48., 16., 16.),
        })
        .with(FollowCamera {
            camera: Camera2D {
                zoom: (4. / screen_width(), 4. / screen_height()).into(),
                ..Default::default()
            }
        })
        .with(ServerEntityId(eid))
        .build()
}