use hecs::{World, CommandBuffer};
use macroquad::prelude::*;
use ::rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::{graphics::{Graphics, TILE_SIZEF}, unit::Team};

struct Lifetime(f32);

struct Velocity(Vec3);

struct RisingText {
    text: String,
    pos: Vec2,
    velocity: f32,
}

pub fn add_particle(cmd: &mut CommandBuffer, rng: &mut SmallRng, pos: Vec2, color: Color) {
    let vx = rng.gen_range(-0.2 .. 0.2);
    let vy = rng.gen_range(-0.2 .. 0.2);
    let vz = rng.gen_range(0.3 .. 0.7);
    let life = rng.gen_range(4.0 .. 8.0);
    cmd.spawn((
        Vec3::new(pos.x, pos.y, 4.0),
        Velocity(vec3(vx, vy, vz)),
        Lifetime(life),
        color,
    ));
}

pub fn add_unit_capture_particles(cmd: &mut CommandBuffer, mut pos: Vec2, team: Team) {
    let seed = u64::from_be_bytes(macroquad::time::get_time().to_be_bytes());
    let mut rng = SmallRng::seed_from_u64(seed);
    let col = if team == Team::Player { BLUE } else { RED };
    pos += vec2(TILE_SIZEF * 0.5, TILE_SIZEF);

    for _ in 0..10 {
        add_particle(cmd, &mut rng, pos, col);
    }
}

pub fn add_rising_text(cmd: &mut CommandBuffer, s: &str, pos: Vec2) {
    cmd.spawn((RisingText { text: s.to_owned(), pos, velocity: -0.5 }, Lifetime(2.0)));
}

pub fn lerp(a: f32, b: f32, x: f32) -> f32 {
    a + x * (b - a)
}

pub fn update(world: &mut World, delta_time: f32) {
    let mut cmd = CommandBuffer::new();
    for (_, text) in world.query_mut::<&mut RisingText>() {
        text.pos.y += text.velocity;
        text.velocity = lerp(text.velocity, 0.0, 1.0 - 0.002_f32.powf(delta_time));
    }

    for (entity, lifetime) in world.query_mut::<&mut Lifetime>() {
        lifetime.0 -= delta_time;
        if lifetime.0 <= 0.0 {
            cmd.despawn(entity);
        }
    }

    for (_, (pos, vel)) in world.query_mut::<(&mut Vec3, &mut Velocity)>() {
        *pos += vel.0;

        // Gravity
        vel.0.z -= 1.0 * delta_time;

        // Ground collision
        if pos.z < 0.0 {
            pos.z = 0.0;
            vel.0.z = -vel.0.z * 0.5;
            vel.0.x *= 0.5;
            vel.0.y *= 0.5;
        }
    }

    cmd.run_on(world);
}

pub fn draw(world: &mut World, graphics: &Graphics) {
    for (_, (text, lifetime)) in world.query_mut::<(&RisingText, &Lifetime)>() {
        let pos = text.pos.floor();
        let color = if lifetime.0 >= 0.1 { WHITE } else { DARKGRAY };
        graphics.draw_text(text.text.as_str(), pos.x + 1.0, pos.y + 1.0, &BLACK);
        graphics.draw_text(text.text.as_str(), pos.x - 1.0, pos.y - 1.0, &BLACK);
        graphics.draw_text(text.text.as_str(), pos.x, pos.y, &color);
    }
}

pub fn draw_particles(world: &mut World, graphics: &Graphics) {
    for (_, (pos, col)) in world.query_mut::<(&Vec3, &Color)>() {
        let mut pos = *pos;
        pos.x += graphics.shake.x;
        pos.y += graphics.shake.y;
        // Shadow
        draw_rectangle(pos.x, pos.y, 4.0, 2.0, Color::new(0.0, 0.0, 0.0, 0.5));
        // Particle
        draw_rectangle(pos.x + 1.0, pos.y - pos.z + 1.0, 2.0, 2.0, BLACK);
        draw_rectangle(pos.x, pos.y - pos.z, 2.0, 2.0, *col);
    }
}
