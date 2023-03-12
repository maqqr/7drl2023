use std::{collections::HashMap, default::default};
use macroquad::prelude::*;
use enum_iterator::{Sequence, all};

use crate::unit::{Unit, Team, UnitType};

pub const TILE_SIZE: usize = 20;
pub const TILE_SIZEF: f32 = TILE_SIZE as f32;
pub const ANIM_STOP_FRAMES: usize = 4;

#[derive(Default)]
pub struct MouseInfo {
    pub old_pos: Vec2,
    pub pos: Vec2,
    pub delta: Vec2,

    pub old_tile: IVec2,
    pub tile: IVec2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Sequence)]
pub enum Sprite {
    Horse(bool),
    Pawn(bool),
    Bishop(bool),
    Jester(bool),
    Rook(bool),
    King(bool),
    Queen(bool),
    Archbishop(bool),
    TileLight,
    TileDark,
    Stairs,
    Shadow,
}

fn sprite_texture_name(sprite: Sprite) -> &'static str {
    match sprite {
        Sprite::Horse(false) => "horse_red_8",
        Sprite::Pawn(false) => "pawn_red_8",
        Sprite::Bishop(false) => "bishop_red_8",
        Sprite::Jester(false) => "jester_red_8",
        Sprite::Rook(false) => "rook_red_8",
        Sprite::King(false) => "king_red_8",
        Sprite::Queen(false) => "queen_red_8",
        Sprite::Archbishop(false) => "archbishop_red_8",

        Sprite::Horse(true) => "horse_blue_8",
        Sprite::Pawn(true) => "pawn_blue_8",
        Sprite::Bishop(true) => "bishop_blue_8",
        Sprite::Jester(true) => "jester_blue_8",
        Sprite::Rook(true) => "rook_blue_8",
        Sprite::King(true) => "king_blue_8",
        Sprite::Queen(true) => "queen_blue_8",
        Sprite::Archbishop(true) => "archbishop_blue_8",

        Sprite::TileLight => "light",
        Sprite::TileDark => "dark",
        Sprite::Stairs => "stairs",
        Sprite::Shadow => "shadow",
    }
}

pub fn unit_into_sprite(unit: &Unit) -> Sprite {
    match unit.unit_type {
        UnitType::Pawn => Sprite::Pawn(unit.team == Team::Player),
        UnitType::Knight => Sprite::Horse(unit.team == Team::Player),
        UnitType::King => Sprite::King(unit.team == Team::Player),
        UnitType::Bishop => Sprite::Bishop(unit.team == Team::Player),
        UnitType::Jester => Sprite::Jester(unit.team == Team::Player),
        UnitType::Rook => Sprite::Rook(unit.team == Team::Player),
        UnitType::Queen => Sprite::Queen(unit.team == Team::Player),
        UnitType::Archbishop => Sprite::Archbishop(unit.team == Team::Player),
    }
}

fn extract_count_from_path(path: &str) -> Option<usize> {
    let mut parts = path.split('.');
    let num_str = parts.next()?.split('_').last()?;
    num_str.parse().ok()
}

const CHARACTER_FRAMES: usize = ANIM_STOP_FRAMES * 2 + 3 + 3;

pub const CHARACTER_ANIM_INDICES: [usize; CHARACTER_FRAMES] = const {
    let mut v = [0usize; CHARACTER_FRAMES];
    let mut index = 0;

    let mut count = 0;
    while count < ANIM_STOP_FRAMES {
        v[index] = 0;
        index += 1;
        count += 1;
    }

    v[index] = 1;
    v[index + 1] = 2;
    v[index + 2] = 3;
    index += 3;

    count = 0;
    while count < ANIM_STOP_FRAMES {
        v[index] = 4;
        index += 1;
        count += 1;
    }

    v[index] = 3;
    v[index + 1] = 2;
    v[index + 2] = 1;
    v
};

#[derive(Debug)]
struct SpriteAtlas {
    texture: Texture2D,
    count: usize,
    sprite_width: f32,
    sprite_height: f32,
}

pub struct Graphics {
    pub time: f32,
    textures: HashMap<Sprite, SpriteAtlas>,
    pub font: Font,
    pub shake: Vec2,
}

impl Graphics {
    pub async fn new() -> Self {
        let mut textures = HashMap::new();
        for spr in all::<Sprite>() {
            let texture_path = sprite_texture_name(spr);
            textures.insert(spr, Graphics::load_tex(format!("assets/{texture_path}.png").as_str()).await);
        }
        Graphics {
            time: 0.0,
            textures,
            font: load_ttf_font("assets/CompassPro.ttf").await.unwrap(),
            shake: Vec2::ZERO,
        }
    }

    async fn load_tex(path: &str) -> SpriteAtlas {
        miniquad::info!("Loading {}...", path);
        let count = extract_count_from_path(path).unwrap_or(1);
        match load_texture(path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                SpriteAtlas {
                    texture: tex,
                    count,
                    sprite_width: tex.width() / count as f32,
                    sprite_height: tex.height(),
                }
            }
            _ => {
                miniquad::error!("Failed to load texture {}", path);
                panic!();
            }
        }
    }

    pub fn highlight_square_alpha(&self, (offset_x, offset_y): (f32, f32), p: &IVec2, mut color: Color) {
        color.a = 0.5;
        draw_rectangle(
            offset_x + p.x as f32 * TILE_SIZEF,
            offset_y + p.y as f32 * TILE_SIZEF,
            TILE_SIZEF, TILE_SIZEF,
            color
        );
    }

    pub fn highlight_square(&self, (offset_x, offset_y): (f32, f32), p: &IVec2, color: Color) {
        draw_rectangle_lines(offset_x - 1.0 + p.x as f32 * TILE_SIZEF,
                             offset_y - 1.0 + p.y as f32 * TILE_SIZEF,
                             TILE_SIZEF + 2.0, TILE_SIZEF + 2.0, 4.0, color);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tex_ex(&self, texture: Texture2D, x: f32, y: f32, w: f32, h: f32, rotation: f32, color: Color, outline: bool) {
        if outline {
            for dy in -1..2 {
                for dx in -1..2 {
                    draw_texture_ex(texture, x + dx as f32, y + dy as f32, BLACK, DrawTextureParams { dest_size: Some(Vec2::new(w, h)), rotation, ..default() });
                }
            }
        }
        draw_texture_ex(texture, x, y, color, DrawTextureParams { dest_size: Some(Vec2::new(w, h)), rotation, ..default() });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite_ex(&self, sprite: Sprite, x: f32, y: f32, w: f32, h: f32, color: &Color, outline: bool) {
        let atlas = self.textures.get(&sprite).unwrap();
        self.draw_tex_ex(atlas.texture, x, y, w, h, 0.0, *color, outline);
    }

    pub fn draw_sprite(&self, sprite: Sprite, index: usize, x: f32, y: f32, color: &Color, outline: bool) {
        let atlas = self.textures.get(&sprite).unwrap();

        let source = Some(Rect {
            x: (index % atlas.count) as f32 * atlas.sprite_width,
            y: 0.0,
            w: atlas.sprite_width,
            h: atlas.sprite_height
        });
        let dest_size = Some(Vec2::new(atlas.sprite_width, atlas.sprite_height));

        if outline {
            for dy in -1..2 {
                for dx in -1..2 {
                    draw_texture_ex(
                        atlas.texture,
                        x + dx as f32,
                        y + dy as f32,
                        BLACK,
                        DrawTextureParams { source, dest_size, rotation: 0.0, ..default() });
                }
            }
        }
        draw_texture_ex(atlas.texture, x, y, *color, DrawTextureParams { source, dest_size, rotation: 0.0, ..default() });
    }

    pub fn draw_text(&self, s: &str, x: f32, y: f32, color: &Color) {
        draw_text_ex(s, x, y,
            TextParams {
                font: self.font,
                font_size: 16,
                font_scale: 1.0,
                color: *color,
                ..default()
        });
    }

    pub fn draw_large_text(&self, s: &str, x: f32, y: f32, color: &Color) {
        draw_text_ex(s, x, y,
            TextParams {
                font: self.font,
                font_size: 16 * 2,
                font_scale: 1.0,
                color: *color,
                ..default()
        });
    }

    pub fn draw_button(&self, s: &str, x: f32, y: f32, mouse: &MouseInfo) -> bool {
        let margin = 5.0;
        let dimensions = measure_text(s, Some(self.font), 16, 1.0);
        let click_area = Rect::new(x - margin - 2.0, y - margin - 2.0, dimensions.width + 2.0 * margin + 4.0, dimensions.height + 2.0 * margin + 4.0);
        let is_mouse_inside = click_area.contains(mouse.pos);
        let color = if is_mouse_inside { WHITE } else { LIGHTGRAY };

        draw_rectangle(x - margin, y - margin, dimensions.width + 2.0 * margin, dimensions.height + 2.0 * margin, BLACK);
        draw_rectangle_lines(x - margin - 2.0, y - margin, dimensions.width + 2.0 * margin + 4.0, dimensions.height + 2.0 * margin, 4.0, color);
        draw_rectangle_lines(x - margin, y - margin - 2.0, dimensions.width + 2.0 * margin, dimensions.height + 2.0 * margin + 4.0, 4.0, color);
        self.draw_text(s, x, y + dimensions.offset_y, &color);

        is_mouse_inside && macroquad::input::is_mouse_button_pressed(MouseButton::Left)
    }
}
