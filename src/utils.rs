use hecs::{World, CommandBuffer};
use macroquad::prelude::{IVec2, Vec2};

use crate::graphics::TILE_SIZEF;

pub fn dist(v1: &IVec2, v2: &IVec2) -> f32 {
    f32::sqrt(dist2(v1, v2))
}

pub fn dist2(v1: &IVec2, v2: &IVec2) -> f32 {
    ((v1.x - v2.x) * (v1.x - v2.x) + (v1.y - v2.y) * (v1.y - v2.y)) as f32
}

pub fn delete_all_components<T: Send + Sync + 'static>(world: &mut World) {
    let mut cmd = CommandBuffer::new();
    for (e, _) in world.query_mut::<&T>() {
        cmd.remove_one::<T>(e);
    }
    cmd.run_on(world);
}

pub fn smootherstep(mut x: f32) -> f32 {
    x = x.clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

pub fn halfcircle(x: f32) -> f32 {
    2.0 * (0.5 * 0.5 - (x - 0.5) * (x - 0.5)).sqrt()
}

pub fn tile_pos_to_pixels(p: &IVec2) -> Vec2 {
    Vec2::new(p.x as f32 * TILE_SIZEF + 2.0, (p.y - 1) as f32 * TILE_SIZEF + 6.0)
}
