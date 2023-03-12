use macroquad::prelude::{Rect, IVec2, ivec2};
use rand::{rngs::SmallRng, Rng};

use crate::{tile::{TileMap, Tile}, utils};

struct RectIterator {
    width: i32,
    height: i32,
    offset: IVec2,
    pos: IVec2,
}

impl Iterator for RectIterator {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos.x += 1;
        if self.pos.x >= self.width {
            self.pos.x = 0;
            self.pos.y += 1;
        }
        if self.pos.y >= self.height {
            return None
        }
        Some(self.offset + self.pos)
    }
}

fn iterate_rect(rect: &Rect) -> RectIterator {
    RectIterator {
        width: rect.w as i32,
        height: rect.h as i32,
        offset: rect.point().as_ivec2(),
        pos: ivec2(-1, 0),
    }
}

fn gen_ivec2(rng: &mut SmallRng, min: i32, max: i32) -> IVec2 {
    let x = rng.gen_range(min..max);
    let y = rng.gen_range(min..max);
    ivec2(x, y)
}

fn gen_centered_rect(rng: &mut SmallRng, center: &IVec2, min: i32, max: i32) -> Rect {
    let size = gen_ivec2(rng, min, max);
    let offset = (size.as_vec2() * 0.5).as_ivec2();
    Rect::new((center.x - offset.x) as f32, (center.y - offset.y) as f32, size.x as f32, size.y as f32)
}

fn make_rect(pos: &IVec2, size: &IVec2) -> Rect {
    Rect::new(pos.x as f32, pos.y as f32, size.x as f32, size.y as f32)
}

pub struct MapGenerator {
    pub tilemap: TileMap,
    pub start_pos: IVec2,
    rng: SmallRng,
}

pub struct MapGeneratorResult {
    pub tilemap: TileMap,
    pub start_pos: IVec2,
}

impl MapGenerator {
    pub fn new(rng: SmallRng, width: usize, height: usize) -> Self {
        MapGenerator {
            tilemap: TileMap::new(width, height),
            start_pos: IVec2::ZERO,
            rng,
        }
    }

    fn make_room(&mut self, rect: &Rect) {
        iterate_rect(rect).for_each(|p| self.tilemap.set(p, Tile::Floor));
    }

    fn can_place_room(&self, rect: &Rect) -> bool {
        iterate_rect(rect).all(|p| self.tilemap.get(p) == Some(Tile::Empty))
    }

    pub fn random_tile_pos(&mut self) -> IVec2 {
        let x = self.rng.gen_range(0..self.tilemap.get_width());
        let y = self.rng.gen_range(0..self.tilemap.get_width());
        ivec2(x as i32, y as i32)
    }

    fn find_room_edge(&mut self) -> Option<IVec2> {
        static DELTA: [IVec2; 5] = [ivec2(0, 0), ivec2(-1, 0), ivec2(1, 0), ivec2(0, -1), ivec2(0, 1)];
        for _ in 0..100 {
            let pos = self.random_tile_pos();
            let is_edge = DELTA.iter().filter(|d| self.tilemap.get(pos + **d) == Some(Tile::Floor)).count() == 1;
            if is_edge {
                return Some(pos)
            }
        }
        None
    }

    fn try_generate(&mut self) -> bool {
        let size = ivec2(5, 5);
        self.start_pos = ivec2(self.rng.gen_range(0..=(self.tilemap.get_width() as i32 - size.x)), self.rng.gen_range(0..=(self.tilemap.get_height() as i32 - size.y)));

        let start_room = make_rect(&self.start_pos, &size);
        assert!(self.can_place_room(&start_room));

        self.make_room(&make_rect(&self.start_pos, &size));

        const ROOMS: usize = 5;
        let mut generated_room_count = 0;
        for _ in 0..100 {
            if let Some(edge_pos) = self.find_room_edge() {
                let room = make_rect(&edge_pos, &gen_ivec2(&mut self.rng, 2, 8));
                if self.can_place_room(&room) {
                    self.make_room(&room);
                    generated_room_count += 1;
                }
            }

            if generated_room_count >= ROOMS {
                break;
            }
        }

        if generated_room_count < ROOMS {
            return false
        }

        let mut potential_stairs = vec![];
        for _ in 0..100 {
            let pos = self.random_tile_pos();
            if self.tilemap.get(pos) == Some(Tile::Floor) {
                potential_stairs.push(pos);
            }
        }

        if potential_stairs.is_empty() {
            return false
        }

        potential_stairs.sort_by_cached_key(|p| utils::dist2(p, &self.start_pos) as i32);

        self.tilemap.set(*potential_stairs.last().unwrap(), Tile::Stairs);
        self.start_pos += ivec2(2, 2);
        true
    }

    pub fn generate(&mut self) -> MapGeneratorResult {
        loop {
            self.tilemap = TileMap::new(self.tilemap.get_width(), self.tilemap.get_height());
            if self.try_generate() {
                return MapGeneratorResult {
                    tilemap: self.tilemap.clone(),
                    start_pos: self.start_pos,
                };
            }
        }
    }
}
