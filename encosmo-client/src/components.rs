// Any component in this file is a client-component, meaning it's
// specific to client-side operation and should never been sent to the server.

use macroquad::prelude::*;
use specs::prelude::*;

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