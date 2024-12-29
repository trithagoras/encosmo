use macroquad::prelude::*;
use specs::prelude::*;


#[derive(Debug, Default)]
pub struct Pos {
    pub x: i32,
    pub y: i32
}

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

#[derive(Debug, Default)]
pub struct Translate {
    pub dx: i32,
    pub dy: i32
}

impl Component for Translate {
    type Storage = VecStorage<Self>;
}

pub struct PlayerInput;

impl Component for PlayerInput {
    type Storage = VecStorage<Self>;
}

pub struct Render {
    pub texture: Texture2D,
    pub source: Rect
}

impl Component for Render {
    type Storage = VecStorage<Self>;
}

pub struct FollowCamera {
    pub camera: Camera2D
}

impl Component for FollowCamera {
    type Storage = VecStorage<Self>;
}