use specs::prelude::*;
use encosmo_shared::server_components::*;

pub struct MoveSystem;

impl<'a> System<'a> for MoveSystem {
    type SystemData = (WriteStorage<'a, Position>, ReadStorage<'a, Translate>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        for (pos, vel) in (&mut pos, &vel).join() {
            pos.x += vel.dx;
            pos.y += vel.dy;
        }
    }
}