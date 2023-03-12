#![windows_subsystem = "windows"]
#![feature(default_free_fn)]
#![feature(inline_const)]
mod graphics;
mod unit;
mod tile;
mod evaluation;
mod utils;
mod boardstate;
mod mapgenerator;
mod effects;
mod sound;

use hecs::{World, Entity, CommandBuffer};
use macroquad::prelude::*;
use once_cell::sync::Lazy;
use ::rand::{rngs::SmallRng, SeedableRng, Rng};
use std::{default::default, borrow::Cow};
use glam::i32::ivec2;
use unit::*;
use tile::*;
use graphics::*;
use evaluation::*;
use boardstate::*;
use mapgenerator::*;
use sound::*;

const SCREEN_SIZE: Vec2 = Vec2::new(384.0, 384.0);

const LAST_FLOOR: usize = 12;

fn draw_board(graphics: &Graphics, Vec2 { x: offset_x, y: offset_y }: Vec2, tilemap: &TileMap) {
    for y in 0..tilemap.get_height() {
        for x in 0..tilemap.get_width() {
            let tile = tilemap.get_unchecked(ivec2(x as i32, y as i32));
            if tile == Tile::Empty {
            }
            else if tile == Tile::Floor {
                let floor_tex = if (x + y) % 2 == 0 { Sprite::TileDark } else { Sprite::TileLight };
                graphics.draw_sprite(floor_tex, 0, offset_x + (x * TILE_SIZE) as f32, offset_y + (y * TILE_SIZE) as f32, &WHITE, false);
            }
            else if tile == Tile::Stairs {
                graphics.draw_sprite(Sprite::Stairs, 0, offset_x + (x * TILE_SIZE) as f32, offset_y + (y * TILE_SIZE) as f32, &WHITE, false);
            }
            else {
                let col = get_tile_color(tile);
                graphics.draw_sprite(Sprite::TileLight, 0, offset_x + (x * TILE_SIZE) as f32, offset_y + (y * TILE_SIZE) as f32, col, false);
            }
        }
    }
}

fn draw_units(graphics: &Graphics, Vec2 { x: offset_x, y: offset_y }: Vec2, world: &World) {
    let char_sprite_index = CHARACTER_ANIM_INDICES[(graphics.time * 16.0) as usize % CHARACTER_ANIM_INDICES.len()];
    let mut q = world.query::<(&Vec3, &Unit)>();
    let mut units = q.iter().map(|(_, data)| data).collect::<Vec<_>>();
    units.sort_by(|(a, _), (b, _)| a.y.partial_cmp(&b.y).unwrap());
    for (Vec3 { x, y, z }, unit) in units.into_iter() {
        graphics.draw_sprite_ex(Sprite::Shadow, offset_x + x, offset_y + y + 27.0, 16.0, 8.0, &BLACK, false);
        graphics.draw_sprite(unit_into_sprite(unit), char_sprite_index, offset_x + x, offset_y + y + z, &WHITE, true);
    }
}

#[derive(Default, Clone, Copy)]
struct InitialPosition {
    offset: IVec2, // Offset in relation to player's king
}

#[derive(Default, Clone, Copy)]
struct SelectedUnit;

#[derive(Default, Clone, Copy)]
struct ShopDrag {
    mouse: Vec2,
}

#[derive(Default, Clone, Copy)]
struct UnitAnimation {
    start_time: f64,
    duration: f64,
    unit_move: Move,
    captured_unit: Option<Entity>,
}
impl UnitAnimation {
    fn new(m: Move, captured_unit: Option<Entity>) -> Self {
        let duration = 0.2 + 0.1 * utils::dist(&m.from, &m.to) as f64;
        UnitAnimation {
            unit_move: m,
            start_time: macroquad::time::get_time(),
            duration,
            captured_unit,
        }
    }
}

struct ShopState {
    board_offset: Vec2,
    board_anim: Option<Vec2>,
}
impl ShopState {
    fn new() -> Self {
        ShopState { board_offset: vec2(42.0, 70.0), board_anim: None }
    }
}

struct GameState {
    rng: SmallRng,
    material: i32,
    board_offset: Vec2,
    player_turn: bool,
    tilemap: TileMap,
    world: World,
    valid_moves_for_selected_unit: Vec<Move>,
    highlighted_moves: Vec<Move>,
    highlighted_unit: Unit,
    is_shopping: bool,
    win_timer: Option<f32>,
    gameover_timer: Option<f32>,
    floor: usize,
    last_gen_result: Option<MapGeneratorResult>,
    shop_state: ShopState,
    camera_shake: f32,
}

impl GameState {
    fn new() -> Self {
        let seed = u64::from_be_bytes(macroquad::time::get_time().to_be_bytes());
        GameState {
            rng: SmallRng::seed_from_u64(seed),
            material: 20,
            board_offset: Vec2::new(42.0, 70.0),
            player_turn: true,
            tilemap: TileMap::new(1, 1),
            world: World::new(),
            valid_moves_for_selected_unit: default(),
            highlighted_moves: default(),
            highlighted_unit: default(),
            is_shopping: false,
            win_timer: None,
            gameover_timer: None,
            floor: 0,
            last_gen_result: None,
            shop_state: ShopState::new(),
            camera_shake: 0.0,
        }
    }

    fn get_valid_moves(&self, entity: Entity) -> Vec<Move> {
        let mut moves = vec![];
        if let Some(selected_unit) = self.world.query_one::<&Unit>(entity).unwrap().get() {
            moves = self.get_boardstate().get_valid_moves_for_unit(selected_unit);
        }
        moves
    }

    fn select_unit(&mut self, selection: Option<Entity>) {
        self.valid_moves_for_selected_unit.clear();
        utils::delete_all_components::<SelectedUnit>(&mut self.world);

        if let Some(entity) = selection {
            assert!(self.world.insert_one(entity, SelectedUnit::default()).is_ok());
            self.valid_moves_for_selected_unit = self.get_valid_moves(entity);
        }
    }

    fn get_selected_unit(&self) -> Option<(Entity, Unit, SelectedUnit)> {
        self.world.query::<(&Unit, &SelectedUnit)>().iter().next().map(|(e, (u, s))| (e, *u, *s))
    }

    fn add_unit(&mut self, pos: IVec2, unit_type: UnitType, team: Team, offset: Option<InitialPosition>) {
        let unit_entity = self.world.spawn((
            Vec3::ZERO,
            Unit { pos, unit_type, jester_type: UnitType::Rook, team },
            UnitAnimation::new(Move { from: pos, to: pos }, None)
        ));
        if let Some(offset) = offset {
            assert!(self.world.insert_one(unit_entity, offset).is_ok());
        }
    }

    fn get_boardstate(&self) -> BoardState {
        let stairs = self.tilemap.find_tile(Tile::Stairs);
        BoardState {
            tilemap: Cow::Borrowed(&self.tilemap),
            units: self.world.query::<&Unit>().iter().map(|(_, u)| *u).collect::<Vec<_>>(),
            stairs,
        }
    }

    fn make_ai_move(&mut self) {
        assert!(!self.player_turn);
        let state = self.get_boardstate();
        let eval = Evaluation::from_gamestate(state);
        if let Some(next_move) = eval.minimax(5, f32::MIN, f32::MAX, false, &mut vec![]).0 {
            let (entity, unit) = self.get_unit_at(&next_move.from).unwrap();
            assert_eq!(unit.team, Team::Ai);
            self.make_move(entity, &next_move);
        }
        self.player_turn = true;
    }

    fn make_player_move(&mut self, entity: Entity, m: &Move) {
        assert!(self.player_turn);
        self.make_move(entity, m);
        self.player_turn = false;
    }

    fn get_unit_at(&self, tile_pos: &IVec2) -> Option<(Entity, Unit)> {
        let mut q = self.world.query::<&Unit>();
        let x = q.iter().find(|(_, u)| u.pos == *tile_pos);
        x.map(|(e, u)| (e, *u))
    }

    fn make_move(&mut self, entity: Entity, m: &Move) {
        let captured = self.get_unit_at(&m.to);
        let captured_entity = captured.map(|(e, _)| e);
        let captured_unit = captured.map(|(_, u)| u);

        let mut unit = self.world.query_one_mut::<&mut Unit>(entity).unwrap();
        unit.pos = m.to;
        if let Some(captured_unit) = captured_unit {
            if unit.unit_type == UnitType::Jester {
                unit.convert_jester(captured_unit);
            }
            if captured_unit.team == Team::Ai {
                self.material += material_reward(captured_unit.unit_type);
            }

            if captured_unit.unit_type == UnitType::King && captured_unit.team == Team::Player {
                self.gameover_timer = Some(4.0);
            }

            if captured_unit.unit_type == UnitType::King && captured_unit.team == Team::Ai {
                self.win_timer = Some(4.0);
            }
        }
        assert!(self.world.insert_one(entity, UnitAnimation::new(*m, captured_entity)).is_ok());
    }

    fn get_mouse_tile(&self, camera: &Camera2D) -> IVec2 {
        let raw_mouse = macroquad::input::mouse_position();
        let mut m = camera.screen_to_world(raw_mouse.into());
        m.y = (m.y * -1.0 + 0.5) * SCREEN_SIZE.x;
        m.x = (m.x + 0.5) * SCREEN_SIZE.y;
        m.x = (m.x - self.board_offset.x) / TILE_SIZEF;
        m.y = (m.y - self.board_offset.y) / TILE_SIZEF;
        ivec2(m.x as i32, m.y as i32)
    }

    fn get_mouse(&self, camera: &Camera2D) -> Vec2 {
        let raw_mouse = macroquad::input::mouse_position();
        let mut m = camera.screen_to_world(raw_mouse.into());
        m.y = (m.y * -1.0 + 0.5) * SCREEN_SIZE.x;
        m.x = (m.x + 0.5) * SCREEN_SIZE.y;
        m
    }

    fn get_random_empty_tile(&mut self) -> Option<IVec2> {
        for _ in 0..200 {
            let x = self.rng.gen_range(0..self.tilemap.get_width());
            let y = self.rng.gen_range(0..self.tilemap.get_width());
            let pos = ivec2(x as i32, y as i32);

            if self.tilemap.get(pos) == Some(Tile::Floor) && self.get_unit_at(&pos).is_none() {
                return Some(pos)
            }
        }
        None
    }

    fn get_random_empty_away_from_spawn(&mut self, spawn_pos: IVec2) -> Option<IVec2> {
        for _ in 0..100 {
            if let Some(pos) = self.get_random_empty_tile() {
                if utils::dist2(&pos, &spawn_pos) > 4.0f32.powi(2) {
                    return Some(pos)
                }
            }
        }
        None
    }

    fn collect_player_units(&self) -> Vec<(UnitType, InitialPosition)> {
        let mut q = self.world.query::<(&Unit, &InitialPosition)>();
        q.iter().map(|(_, (u, p))| (u.unit_type, *p)).collect::<Vec<_>>()
    }

    fn show_shop(&mut self) {
        let units = self.collect_player_units();
        self.world.clear();
        self.is_shopping = true;
        self.shop_state = ShopState::new();

        for (unit_type, initial) in units {
            self.add_unit(ivec2(2, 2) + initial.offset, unit_type, Team::Player, Some(initial));
        }
    }

    fn pre_generate_next_floor(&mut self) {
        let mut gen = MapGenerator::new(self.rng.clone(), 15, 15);
        self.last_gen_result = Some(gen.generate());
    }

    fn generate_next_floor(&mut self, player_units: &[(UnitType, InitialPosition)]) {
        self.floor += 1;

        self.tilemap = self.last_gen_result.as_ref().unwrap().tilemap.clone();
        let start_pos = self.last_gen_result.as_ref().unwrap().start_pos;

        self.player_turn = true;
        self.valid_moves_for_selected_unit.clear();
        self.highlighted_moves.clear();

        self.world.clear();
        self.is_shopping = false;

        for &(unit_type, initial) in player_units {
            self.add_unit(start_pos + initial.offset, unit_type, Team::Player, Some(initial));
        }

        let mut enemy_units = vec![];
        let mut enemy_material = 1 + 2 * self.floor as i32;
        let unit_list = if self.floor <= 2 {
            &[UnitType::Pawn][..]
        }
        else if self.floor <= 4 {
            &[UnitType::Pawn, UnitType::Knight, UnitType::Bishop][..]
        }
        else if self.floor <= 6 {
            &[UnitType::Pawn, UnitType::Knight, UnitType::Bishop, UnitType::Archbishop][..]
        }
        else {
            &[UnitType::Pawn, UnitType::Knight, UnitType::Bishop, UnitType::Jester, UnitType::Rook][..]
        };

        while enemy_material > 0 {
            let available = unit_list.iter().filter(|u| enemy_material >= unit_buy_price(**u)).collect::<Vec<_>>();
            let index = self.rng.gen_range(0..available.len());
            let unit = available[index];
            enemy_material -= unit_buy_price(*unit);
            enemy_units.push(*unit);
        }

        if self.floor == LAST_FLOOR {
            enemy_units.push(UnitType::King);
        }

        while let Some(unit_type) = enemy_units.pop() {
            if unit_type == UnitType::King {
                if let Some(p) = self.tilemap.find_tile(Tile::Stairs) {
                    self.add_unit(p, unit_type, Team::Ai, None);
                    self.tilemap.set(p, Tile::Floor);
                }
            }
            else if let Some(pos) = self.get_random_empty_away_from_spawn(start_pos) {
                self.add_unit(pos, unit_type, Team::Ai, None);
            }
        }
    }
}

fn animate_units(world: &mut World, board_offset: &Vec2, camera_shake: &mut f32, sound: &Sound) -> bool {
    let mut animation_going = false;

    let now = macroquad::time::get_time();
    let mut cmd = CommandBuffer::new();

    let mut destroyed_entities = vec![];

    for (entity, (pos, unit, anim)) in world.query_mut::<(&mut Vec3, &mut Unit, &mut UnitAnimation)>() {
        let progress = ((now - anim.start_time) / anim.duration) as f32;
        let end = utils::tile_pos_to_pixels(&anim.unit_move.to);
        if progress >= 1.0 {
            cmd.remove_one::<UnitAnimation>(entity);
            *pos = Vec3::new(end.x, end.y, 0.0);
            // Delete unit at target tile when animation finishes
            if let Some(entity) = anim.captured_unit {
                cmd.despawn(entity);
                destroyed_entities.push(entity);
            }
            if unit.team == Team::Ai {
                sound.play("thud");
            }
        }
        else {
            let delta = anim.unit_move.to - anim.unit_move.from;
            let is_diagonal = delta.x.abs() == delta.y.abs();
            let jump_height: f32 =
                if unit.unit_type == UnitType::Knight || unit.jester_type == UnitType::Knight || (unit.unit_type == UnitType::Archbishop && !is_diagonal) {
                    25.0
                }
                else {
                    5.0
                };
            let smooth_progress = utils::smootherstep(progress);
            let start = utils::tile_pos_to_pixels(&anim.unit_move.from);

            let pos2 = start.lerp(end, smooth_progress);
            pos.x = pos2.x;
            pos.y = pos2.y;
            pos.z = -jump_height * utils::halfcircle(smooth_progress);
        }
        animation_going = true;
    }

    for entity in destroyed_entities {
        if let Ok((pos, unit)) = world.query_one_mut::<(&Vec3, &Unit)>(entity) {
            let effect_pos = pos.xy() + *board_offset;
            effects::add_unit_capture_particles(&mut cmd, effect_pos, unit.team);
            if unit.team == Team::Ai {
                effects::add_rising_text(&mut cmd, format!("+{}", material_reward(unit.unit_type)).as_str(), effect_pos);
            }
            *camera_shake = 4.0;
            sound.play("thud2");
        }
    }

    cmd.run_on(world);
    animation_going
}

fn start_new_game() -> GameState {
    let mut gamestate = GameState::new();
    let units = [(UnitType::King, ivec2(0, 0)), (UnitType::Knight, ivec2(1, 0)), (UnitType::Bishop, ivec2(-1, 0))];
    gamestate.pre_generate_next_floor();
    gamestate.generate_next_floor(units.map(|(u, p)| (u, InitialPosition{ offset: p })).as_slice());
    gamestate
}

fn game_loop(gamestate: &mut GameState, graphics: &mut Graphics, mouse: &MouseInfo, sound: &Sound) {
    let is_animation_going = animate_units(&mut gamestate.world, &gamestate.board_offset, &mut gamestate.camera_shake, sound);
    let player_can_act = gamestate.player_turn && !is_animation_going && gamestate.gameover_timer.is_none() && gamestate.win_timer.is_none();

    let state = gamestate.get_boardstate();
    if state.is_on_stairs() {
        if !is_animation_going {
            gamestate.show_shop();
            sound.play("stairs");
        }
    }
    else if !gamestate.player_turn && !is_animation_going {
        gamestate.make_ai_move();
    }

    draw_rectangle_lines(5.0, 65.0, SCREEN_SIZE.x - 10.0, SCREEN_SIZE.y - 70.0, 4.0, WHITE);

    let shaken_offset = gamestate.board_offset + graphics.shake;
    draw_board(graphics, shaken_offset, &gamestate.tilemap);

    effects::draw_particles(&mut gamestate.world, graphics);

    // Detect when mouse changes tile and highlight moves
    if mouse.old_tile != mouse.tile {
        if let Some((entity, unit)) = gamestate.get_unit_at(&mouse.tile) {
            gamestate.highlighted_moves = gamestate.get_valid_moves(entity);
            gamestate.highlighted_unit = unit;
        }
        else {
            gamestate.highlighted_moves.clear();
        }
    }

    if player_can_act {
        // Highlight unit moves
        for m in &gamestate.highlighted_moves {
            let col = if gamestate.highlighted_unit.team == Team::Player { DARKBLUE } else { RED };
            graphics.highlight_square_alpha(gamestate.board_offset.into(), &m.to, col);
        }

        // Highlight possible moves
        for m in &gamestate.valid_moves_for_selected_unit {
            graphics.highlight_square(gamestate.board_offset.into(), &m.to, BLUE);
        }

        // Highlight selected unit
        if let Some((_, unit, _)) = gamestate.get_selected_unit() {
            graphics.highlight_square(gamestate.board_offset.into(), &unit.pos, DARKBLUE);
        }

        // Highlight tile under mouse
        if let Some(tile) = gamestate.tilemap.get(mouse.tile) {
            if tile != Tile::Empty {
                graphics.highlight_square(gamestate.board_offset.into(), &mouse.tile, WHITE);
            }
        }
    }

    draw_units(graphics, shaken_offset, &gamestate.world);

    // Top area UI
    {
        graphics.draw_large_text(if gamestate.player_turn {"Player Turn"} else { "Enemy Turn" }, 10.0, 25.0, &WHITE);
        graphics.draw_text(format!("Material: {}", gamestate.material).as_str(), 200.0, 40.0, &WHITE);
        graphics.draw_large_text(format!("Floor {}", gamestate.floor).as_str(), 200.0, 25.0, &WHITE);
        if !gamestate.highlighted_moves.is_empty() {
            graphics.draw_text(format!("{:?}", gamestate.highlighted_unit.unit_type).as_str(), 10.0, 40.0, &WHITE);
        }
    }

    #[allow(clippy::collapsible_if)]
    if gamestate.gameover_timer.is_none() {
        if graphics.draw_button("Give up", SCREEN_SIZE.x - 55.0, 8.0, mouse) && player_can_act {
            for y in 0..gamestate.tilemap.get_height() {
                for x in 0..gamestate.tilemap.get_width() {
                    if let Some((entity, unit)) = gamestate.get_unit_at(&ivec2(x as i32, y as i32)) {
                        if unit.unit_type == UnitType::King && unit.team == Team::Player {
                            let _ = gamestate.world.despawn(entity);
                            gamestate.gameover_timer = Some(1.0);

                            let pos = utils::tile_pos_to_pixels(&ivec2(x as i32, y as i32)) + gamestate.board_offset;
                            let mut cmd = CommandBuffer::new();
                            effects::add_unit_capture_particles(&mut cmd, pos, unit.team);
                            cmd.run_on(&mut gamestate.world);
                        }
                    }
                }
            }
        }
    }

    if macroquad::input::is_mouse_button_pressed(MouseButton::Left) && player_can_act {
        if let Some((entity, unit)) = gamestate.get_unit_at(&mouse.tile) {
            if unit.team == Team::Player {
                gamestate.select_unit(Some(entity));
                sound.play("thud3");
            }
        }
    }

    if macroquad::input::is_mouse_button_released(MouseButton::Left) && player_can_act {
        if let Some((e, _, _)) = gamestate.get_selected_unit() {
            if let Some(player_move) = gamestate.valid_moves_for_selected_unit.iter().find(|m| m.to == mouse.tile).copied() {
                gamestate.make_player_move(e, &player_move);
                gamestate.select_unit(None);
                sound.play("thud");
            }
        }
    }

    // TODO: remove
    if macroquad::input::is_key_pressed(KeyCode::W) {
        gamestate.show_shop();
    }
}

fn find_pos_for_new_unit(gamestate: &GameState) -> Option<(IVec2, InitialPosition)> {
    for y in 0..5 {
        for x in 0..5 {
            if gamestate.get_unit_at(&ivec2(x, y)).is_none() {
                return Some((ivec2(x, y), InitialPosition { offset: ivec2(x - 2, y - 2) }));
            }
        }
    }
    None
}

fn shop_loop(gamestate: &mut GameState, graphics: &mut Graphics, mouse: &MouseInfo, sound: &Sound) {
    static SHOP_MAP: Lazy<TileMap> = Lazy::new(|| {
        TileMap::from([".....", ".....", ".....", ".....", "....."].as_slice())
    });

    let shop_pieces = [UnitType::Pawn, UnitType::Knight, UnitType::Bishop, UnitType::Jester, UnitType::Rook, UnitType::Archbishop, UnitType::Queen];

    let _ = animate_units(&mut gamestate.world, &gamestate.board_offset, &mut gamestate.camera_shake, sound);

    for (_, (pos, drag)) in gamestate.world.query_mut::<(&mut Vec3, &mut ShopDrag)>() {
        pos.x = drag.mouse.x - gamestate.board_offset.x - TILE_SIZEF * 0.4;
        pos.y = drag.mouse.y - gamestate.board_offset.y - TILE_SIZEF;
        pos.z = 0.0;
    }

    let mut selected_shop_unit = None;
    if gamestate.shop_state.board_anim.is_none() {
        draw_rectangle_lines(5.0, 5.0, SCREEN_SIZE.x - 10.0, SCREEN_SIZE.y - 10.0, 4.0, WHITE);
        draw_rectangle_lines(5.0, 5.0, SCREEN_SIZE.x - 10.0, 185.0, 4.0, WHITE);
        graphics.draw_large_text("Your Units", 20.0, 35.0, &WHITE);

        graphics.draw_text("Drag units to set", 160.0, 85.0, &WHITE);
        graphics.draw_text("their starting positions", 160.0, 100.0, &WHITE);
        graphics.draw_text("Drag unit outside of", 160.0, 130.0, &WHITE);
        graphics.draw_text("the board to sell it", 160.0, 145.0, &WHITE);

        graphics.draw_text("Click unit to buy it", 200.0, 220.0, &WHITE);

        graphics.draw_large_text("Shop", 20.0, 220.0, &WHITE);
        graphics.draw_text(format!("Material: {}", gamestate.material).as_str(), 100.0, 220.0, &WHITE);

        graphics.draw_text("Cost:", 10.0, 300.0, &WHITE);

        for (i, &unit_type) in shop_pieces.iter().enumerate() {
            let x = 50.0 + 2.0 * TILE_SIZEF * i as f32;
            let y = 240.0;
            let color = if gamestate.material >= unit_buy_price(unit_type) { WHITE } else { GRAY };
            graphics.draw_sprite(unit_into_sprite(&Unit { team: Team::Player, unit_type, ..default() }), 0, x, y, &color, true);
            let unit_rect = Rect::new(x, y, TILE_SIZEF, TILE_SIZEF * 2.0);
            if unit_rect.contains(mouse.pos) {
                selected_shop_unit = Some(unit_type);
                graphics.draw_text(format!("{unit_type:?}").as_str(), 10.0, 320.0, &WHITE);
                graphics.draw_text(unit_description(unit_type), 10.0, 340.0, &WHITE);
            }

            graphics.draw_text(unit_buy_price(unit_type).to_string().as_str(), x + 3.0, y + TILE_SIZEF * 3.0, &color);
        }

        if graphics.draw_button("Start next level", SCREEN_SIZE.x - 120.0, SCREEN_SIZE.y - 30.0, mouse) {
            gamestate.pre_generate_next_floor();
            let start_pos = utils::tile_pos_to_pixels(&gamestate.last_gen_result.as_ref().unwrap().start_pos);
            gamestate.shop_state.board_anim = Some(start_pos + vec2(0.0, 40.0));
        }
    }

    if let Some(offset) = gamestate.shop_state.board_anim {
        let delta_time = macroquad::time::get_frame_time();
        gamestate.shop_state.board_offset =
            gamestate.shop_state.board_offset.lerp(offset, 1.0 - 0.05_f32.powf(delta_time));

        if gamestate.shop_state.board_offset.distance_squared(offset) < 2.0 * 2.0 {
            let units = gamestate.collect_player_units();
            gamestate.generate_next_floor(&units);
        }
    }

    let target_tile = mouse.tile;
    let is_valid_tile = target_tile.x >= 0 && target_tile.x < 5 && target_tile.y >= 0 && target_tile.y < 5 && !(target_tile.x == 2 && target_tile.y == 2);

    draw_board(graphics, gamestate.shop_state.board_offset, &SHOP_MAP);
    if is_valid_tile && gamestate.shop_state.board_anim.is_none() {
        graphics.highlight_square(gamestate.shop_state.board_offset.into(), &mouse.tile, BLUE);
    }
    draw_units(graphics, gamestate.shop_state.board_offset, &gamestate.world);

    if gamestate.shop_state.board_anim.is_none() {
        if macroquad::input::is_mouse_button_pressed(MouseButton::Left) {
            if is_valid_tile {
                if let Some((entity, _)) = gamestate.get_unit_at(&mouse.tile) {
                    assert!(gamestate.world.insert_one(entity, ShopDrag { mouse: mouse.pos }).is_ok());
                }
            }
            else if let Some(unit_type) = selected_shop_unit {
                if let Some((pos, ipos)) = find_pos_for_new_unit(gamestate) {
                    let price = unit_buy_price(unit_type);
                    if gamestate.material >= price {
                        gamestate.add_unit(pos, unit_type, Team::Player, Some(ipos));
                        gamestate.material -= price;
                    }
                }
            }
        }

        if macroquad::input::is_mouse_button_down(MouseButton::Left) {
            for (_, drag) in gamestate.world.query_mut::<&mut ShopDrag>() {
                drag.mouse = mouse.pos;
            }
        }

        if macroquad::input::is_mouse_button_released(MouseButton::Left) {
            // Find out if an unit needs to swap position
            let mut swap_entity = None;
            let mut swap_to = None;
            if is_valid_tile {
                if let Some((e, _)) = gamestate.get_unit_at(&target_tile) {
                    swap_entity = Some(e);
                }
            }

            // Move dragged unit to its place
            let mut sold_unit = None;
            for (e, (pos, unit, ipos, _)) in gamestate.world.query_mut::<(&mut Vec3, &mut Unit, &mut InitialPosition, &ShopDrag)>() {
                let mut target_tile = target_tile;
                if !is_valid_tile {
                    if target_tile == ivec2(2, 2) {
                        target_tile = unit.pos;
                    }
                    else {
                        sold_unit = Some((e, unit.unit_type));
                    }
                }
                let p = utils::tile_pos_to_pixels(&target_tile);
                pos.x = p.x;
                pos.y = p.y;
                pos.z = 0.0;
                swap_to = Some(unit.pos);
                unit.pos = target_tile;
                ipos.offset = unit.pos - ivec2(2, 2);
            }
            utils::delete_all_components::<ShopDrag>(&mut gamestate.world);

            // Sell unit
            if let Some((entity, unit_type)) = sold_unit {
                gamestate.material += unit_buy_price(unit_type);
                assert!(gamestate.world.despawn(entity).is_ok());
            }

            // Move swapped unit to the dragged unit's original position
            if let Some(swap_entity) = swap_entity {
                if let Some(swap_pos) = swap_to {
                    if let Ok((pos, unit, ipos)) = gamestate.world.query_one_mut::<(&mut Vec3, &mut Unit, &mut InitialPosition)>(swap_entity) {
                        unit.pos = swap_pos;
                        ipos.offset = unit.pos - ivec2(2, 2);
                        let p = utils::tile_pos_to_pixels(&unit.pos);
                        pos.x = p.x;
                        pos.y = p.y;
                        pos.z = 0.0;
                    }
                }
            }
        }
    }
}

fn menu_loop(graphics: &mut Graphics, mouse: &MouseInfo, sound: &Sound) -> bool {
    draw_rectangle_lines(30.0, 120.0, SCREEN_SIZE.x - 60.0, SCREEN_SIZE.y - 240.0, 4.0, WHITE);
    graphics.draw_large_text("King's Conquest", 90.0, 160.0, &WHITE);
    if graphics.draw_button("Click to start", 150.0, 220.0, mouse) {
        sound.play("thud2");
        false
    }
    else {
        true
    }
}

fn gameover_loop(graphics: &mut Graphics, mouse: &MouseInfo, sound: &Sound) -> bool {
    draw_rectangle_lines(30.0, 120.0, SCREEN_SIZE.x - 60.0, SCREEN_SIZE.y - 240.0, 4.0, WHITE);
    graphics.draw_large_text("Game over", 135.0, 160.0, &WHITE);
    if graphics.draw_button("Click to restart", 150.0, 220.0, mouse) {
        sound.play("thud2");
        true
    }
    else {
        false
    }
}

fn win_loop(graphics: &mut Graphics, mouse: &MouseInfo, sound: &Sound) -> bool {
    draw_rectangle_lines(30.0, 120.0, SCREEN_SIZE.x - 60.0, SCREEN_SIZE.y - 240.0, 4.0, WHITE);
    graphics.draw_large_text("You're", 145.0, 160.0, &WHITE);
    graphics.draw_large_text("winner !", 145.0, 180.0, &WHITE);
    if graphics.draw_button("Click to restart", 150.0, 220.0, mouse) {
        sound.play("thud2");
        true
    }
    else {
        false
    }
}

#[macroquad::main("King's Conquest")]
async fn main() {
    let render_target = render_target(SCREEN_SIZE.x as u32, SCREEN_SIZE.y as u32);
    render_target.texture.set_filter(FilterMode::Nearest);

    let ingame_camera = Camera2D {
        render_target: Some(render_target),
        ..Camera2D::from_display_rect(Rect { x: 0.0, y: 0.0, w: SCREEN_SIZE.x, h: SCREEN_SIZE.y })
    };

    let mut final_camera = Camera2D {
        zoom: vec2(2.0, 2.0),
        target: vec2(0.0, 0.0),
        ..default()
    };

    let mut graphics = Graphics::new().await;
    let sound = Sound::new().await;

    let mut mainmenu = true;

    let mut gamestate = start_new_game();
    let mut mouse = MouseInfo::default();

    loop {
        set_camera(&ingame_camera);
        clear_background(BLACK);

        let delta_time = macroquad::time::get_frame_time();
        graphics.time += delta_time;

        mouse.tile = gamestate.get_mouse_tile(&final_camera);
        mouse.pos = gamestate.get_mouse(&final_camera);
        mouse.delta = mouse.pos - mouse.old_pos;

        if gamestate.camera_shake > 0.0 {
            let x = gamestate.rng.gen_range(-1.0..1.0);
            let y = gamestate.rng.gen_range(-1.0..1.0);
            graphics.shake = gamestate.camera_shake * vec2(x, y);

            gamestate.camera_shake = effects::lerp(gamestate.camera_shake, 0.0, 1.0 - 0.01_f32.powf(delta_time));
        }

        let is_gameover = gamestate.gameover_timer.is_some() && gamestate.gameover_timer.unwrap() == 0.0;
        if let Some(time) = &mut gamestate.gameover_timer {
            *time -= delta_time;
            if *time <= 0.0 {
                *time = 0.0;
            }
        }

        let is_win = gamestate.win_timer.is_some() && gamestate.win_timer.unwrap() == 0.0;
        if let Some(time) = &mut gamestate.win_timer {
            *time -= delta_time;
            if *time <= 0.0 {
                *time = 0.0;
            }
        }

        if mainmenu {
            mainmenu = menu_loop(&mut graphics, &mouse, &sound);
        }
        else if is_gameover {
            if gameover_loop(&mut graphics, &mouse, &sound) {
                gamestate = start_new_game();
            }
        }
        else if is_win {
            if win_loop(&mut graphics, &mouse, &sound) {
                gamestate = start_new_game();
            }
        }
        else {
            effects::update(&mut gamestate.world, delta_time);

            if gamestate.is_shopping {
                shop_loop(&mut gamestate, &mut graphics, &mouse, &sound);
            }
            else {
                game_loop(&mut gamestate, &mut graphics, &mouse, &sound);
            }
            effects::draw(&mut gamestate.world, &graphics);
        }


        mouse.old_pos = mouse.pos;

        if screen_width() > screen_height() {
            let aspect = screen_height() / screen_width();
            final_camera.zoom = vec2(2.0 * aspect, 2.0);
        }
        else {
            let aspect = screen_width() / screen_height();
            final_camera.zoom = vec2(2.0, 2.0 * aspect);
        }

        set_camera(&final_camera);
        clear_background(DARKGRAY);
        draw_texture_ex(
            render_target.texture,
            -0.5,
            -0.5,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(1.0, 1.0)),
                ..default()
            },
        );
        next_frame().await
    }
}
