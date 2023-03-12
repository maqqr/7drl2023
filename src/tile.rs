use macroquad::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Tile {
    Empty,
    Floor,
    Wall,
    Stairs,
}

pub fn get_tile_color(tile: Tile) -> &'static Color {
    match tile {
        Tile::Empty => &BLACK,
        Tile::Floor => &WHITE,
        Tile::Wall => &DARKPURPLE,
        Tile::Stairs => &WHITE,
    }
}

pub fn is_passable(tile: Tile) -> bool {
    matches!(tile, Tile::Floor | Tile::Stairs)
}

#[derive(Default, Clone)]
pub struct TileMap {
    tiles: Vec<Tile>,
    width: usize,
    height: usize,
}

impl TileMap {
    pub fn new(width: usize, height: usize) -> Self {
        TileMap { tiles: vec![Tile::Empty; width * height], width, height }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn is_inside(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.x < self.width as i32 && pos.y >= 0 && pos.y < self.height as i32
    }

    pub fn set(&mut self, pos: IVec2, tile: Tile) {
        let (x, y) = (pos.x as usize, pos.y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        self.tiles[x + y * self.width] = tile;
    }

    pub fn get(&self, pos: IVec2) -> Option<Tile> {
        let (x, y) = (pos.x as usize, pos.y as usize);
        if x >= self.width || y >= self.height {
            None
        }
        else {
            Some(*unsafe { self.tiles.get_unchecked(x + y * self.width) })
        }
    }

    pub fn get_unchecked(&self, pos: IVec2) -> Tile {
        let (x, y) = (pos.x as usize, pos.y as usize);
        assert!(x < self.width && y < self.height);
        *unsafe { self.tiles.get_unchecked(x + y * self.width) }
    }

    pub fn is_passable(&self, pos: IVec2) -> bool {
        match self.get(pos) {
            Some(tile) => is_passable(tile),
            None => false,
        }
    }

    pub fn find_tile(&self, tile_type: Tile) -> Option<IVec2> {
        for y in 0..self.height {
            for x in 0..self.width {
                let p = ivec2(x as i32, y as i32);
                if self.get_unchecked(p) == tile_type {
                    return Some(p)
                }
            }
        }
        None
    }
}

impl From<&[&str]> for TileMap {
    fn from(arr: &[&str]) -> Self {
        fn chr_to_tile(chr: char) -> Tile {
            match chr {
                '#' => Tile::Wall,
                '.' => Tile::Floor,
                '<' => Tile::Stairs,
                _ => Tile::Empty,
            }
        }
        let mut tilemap = TileMap::new(arr[0].len(), arr.len());
        for (y, line) in arr.iter().enumerate() {
            for (x, chr) in line.chars().enumerate() {
                tilemap.set(ivec2(x as i32, y as i32), chr_to_tile(chr));
            }
        }
        tilemap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilemap_clone_test() {
        let map_plan = ["...", "###"];
        let mut tilemap = TileMap::from(&map_plan[..]);
        let cloned = tilemap.clone();

        tilemap.set(ivec2(0, 0), Tile::Wall);
        assert_eq!(tilemap.get_unchecked(ivec2(0, 0)), Tile::Wall);
        assert_eq!(cloned.get_unchecked(ivec2(0, 0)), Tile::Floor);
    }
}