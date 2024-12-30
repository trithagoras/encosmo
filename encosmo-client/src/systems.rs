use std::sync::mpsc;

use specs::prelude::*;
use crate::components::*;
use macroquad::prelude::*;
use encosmo_shared::{server_components::*, Packet};


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

pub struct InputSystem {
    pub packet_tx: mpsc::Sender<Packet>
}

impl<'a> System<'a> for InputSystem {
    type SystemData = (
        WriteStorage<'a, Translate>,
        ReadStorage<'a, PlayerInput>,
        ReadStorage<'a, ServerEntityId>
    );

    fn run(&mut self, (mut vel, inp, id): Self::SystemData) {
        for (vel, _, id) in (&mut vel, &inp, &id).join() {
            vel.dx = 0;
            vel.dy = 0;
            if is_key_pressed(KeyCode::Up) {
                vel.dy -= 16;
            }
            else if is_key_pressed(KeyCode::Down) {
                vel.dy += 16;
            }
            else if is_key_pressed(KeyCode::Left) {
                vel.dx -= 16;
            }
            else if is_key_pressed(KeyCode::Right) {
                vel.dx += 16;
            }

            if vel.dx != 0 || vel.dy != 0 {
                let packet = Packet::UpdateComponent(id.0, ServerComponentKind::Translate(vel.clone()));
                _ = self.packet_tx.send(packet);
            }
        }
    }
}

pub struct RenderSystem;
impl<'a> System<'a> for RenderSystem {
    type SystemData = (ReadStorage<'a, Position>, ReadStorage<'a, Render>);

    fn run(&mut self, (pos, render): Self::SystemData) {
        for (pos, render) in (&pos, &render).join() {
            draw_texture_ex(&render.texture, pos.x as f32, pos.y as f32, WHITE, DrawTextureParams {
                dest_size: Some (vec2(16., 16.)),
                source: Some (render.source),
                ..Default::default()
            });
        }
    }
}

pub struct FollowCameraSystem;
impl<'a> System<'a> for FollowCameraSystem {
    type SystemData = (WriteStorage<'a, FollowCamera>, ReadStorage<'a, Position>);

    fn run(&mut self, (mut cam, pos): Self::SystemData) {
        for (cam, pos) in (&mut cam, &pos).join() {
            cam.camera.target = vec2(pos.x as f32, pos.y as f32);
            set_camera(&cam.camera);
        }
    }
}