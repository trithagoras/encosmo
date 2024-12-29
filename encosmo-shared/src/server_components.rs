use serde::{Deserialize, Serialize};
use specs::*;

/// Server components are not exclusive to the server as the name might suggest.
/// Rather, it's used for calculations on the server-side and consumption on the client-side.
/// 
/// # Examples
/// The `Position` of an entity would be a `ServerComponent` since it's used on the client-side for
/// other client components to consume (e.g. the `Render` component needs to know an entity's position)
/// but is also used on the server to calculate collisions, etc.
/// 
/// In contrast, the `Render` component would *not* be a `ServerComponent` since the server
/// doesn't need to know anything about how an entity is rendered.
#[derive(Serialize, Deserialize, Clone)]
pub enum ServerComponentKind {
    Position (Position),
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Translate {
    pub dx: i32,
    pub dy: i32
}

impl Component for Translate {
    type Storage = VecStorage<Self>;
}