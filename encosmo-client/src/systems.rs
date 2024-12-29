use specs::prelude::*;
use crate::components::*;
use macroquad::prelude::*;


pub struct MoveSystem;

impl<'a> System<'a> for MoveSystem {
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Translate>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        for (pos, vel) in (&mut pos, &vel).join() {
            pos.x += vel.dx;
            pos.y += vel.dy;
        }
    }
}

pub struct InputSystem;

impl<'a> System<'a> for InputSystem {
    type SystemData = (WriteStorage<'a, Translate>, ReadStorage<'a, PlayerInput>);

    fn run(&mut self, (mut vel, inp): Self::SystemData) {
        for (vel, _) in (&mut vel, &inp).join() {
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
        }
    }
}

pub struct RenderSystem;
impl<'a> System<'a> for RenderSystem {
    type SystemData = (ReadStorage<'a, Pos>, ReadStorage<'a, Render>);

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
    type SystemData = (WriteStorage<'a, FollowCamera>, ReadStorage<'a, Pos>);

    fn run(&mut self, (mut cam, pos): Self::SystemData) {
        for (cam, pos) in (&mut cam, &pos).join() {
            cam.camera.target = vec2(pos.x as f32, pos.y as f32);
            set_camera(&cam.camera);
        }
    }
}