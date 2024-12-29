use crate::components::*;
use macroquad::prelude::*;
use specs::{Builder, Entity, World, WorldExt};

pub fn create_player(world: &mut World, game_texture: &Texture2D) -> Entity {
    world
        .create_entity()
        .with(Translate::default())
        .with(Pos::default())
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
        .build()
}