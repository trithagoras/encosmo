use specs::prelude::*;
use encosmo_shared::{server_components::*, Packet};

use crate::{messages::Message, resources::ServerTx};

pub struct MoveSystem;

impl<'a> System<'a> for MoveSystem {
    type SystemData = (Entities<'a>, WriteStorage<'a, Position>, WriteStorage<'a, Translate>, ReadExpect<'a, ServerTx>);

    fn run(&mut self, (entities, mut pos, mut trans, res): Self::SystemData) {
        let tx = &res.0;
        for (entity, pos, trans) in (&entities, &mut pos, &mut trans).join() {
            if trans.dx != 0 || trans.dy != 0 {
                pos.x += trans.dx;
                pos.y += trans.dy;

                // reset translate component after update
                *trans = Translate::default();
                let id = entity.id();

                _ = tx.send(Message::SendPacket(Packet::UpdateComponent(id, ServerComponentKind::Translate(trans.clone()))));
            }
        }
    }
}