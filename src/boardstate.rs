use std::borrow::Cow;
use macroquad::prelude::{IVec2, ivec2};
use crate::{tile::TileMap, unit::*};

const PAWN_DELTAS: [IVec2; 4] = [ivec2(0, -1), ivec2(0, 1), ivec2(-1, 0), ivec2(1, 0)];

const KING_DELTAS: [IVec2; 8] = [ivec2(0, -1), ivec2(0, 1), ivec2(-1, 0), ivec2(1, 0),
                                 ivec2(-1, -1), ivec2(1, 1), ivec2(-1, 1), ivec2(1, -1)];

const KNIGHT_DELTAS: [IVec2; 8] = [ivec2(-2, -1), ivec2(-1, -2), ivec2(1, -2), ivec2(2, -1),
                                  ivec2(-2, 1), ivec2(-1, 2), ivec2(1, 2), ivec2(2, 1)];

#[derive(Default, Clone, Copy, Debug)]
pub struct Move {
    pub from: IVec2,
    pub to: IVec2,
}

#[derive(Default)]
pub struct BoardState<'a> {
    pub tilemap: Cow<'a, TileMap>,
    pub units: Vec<Unit>,
    pub stairs: Option<IVec2>,
}

impl<> BoardState<'_> {
    pub fn shallow_clone(&self) -> BoardState<'_> {
        BoardState {
            tilemap: Cow::Borrowed(self.tilemap.as_ref()),
            units: self.units.clone(),
            stairs: self.stairs,
        }
    }

    pub fn get_unit_at(&self, point: &IVec2) -> Option<&Unit> {
        let index = self.units.iter().position(|u| u.pos == *point)?;
        self.units.get(index)
    }

    pub fn get_mut_unit_at(&mut self, point: &IVec2) -> Option<&mut Unit> {
        let index = self.units.iter().position(|u| u.pos == *point)?;
        self.units.get_mut(index)
    }

    pub fn is_valid(&self, m: &Move) -> bool {
        match self.get_unit_at(&m.from) {
            Some(this_unit) => {
                let other_unit_is_enemy = self.get_unit_at(&m.to).map(|u| is_enemy(this_unit, u));
                self.tilemap.is_passable(m.to) && other_unit_is_enemy.unwrap_or(true)
            }
            None => {
                false
            }
        }
    }

    pub fn get_valid_moves_for_unit(&self, unit: &Unit) -> Vec<Move> {
        let mut moves = vec![];

        let mut make_moves_from_deltas = |deltas: &[IVec2]| {
            for delta in deltas {
                let m = Move { from: unit.pos, to: unit.pos + *delta };
                if self.is_valid(&m) {
                    moves.push(m);
                }
            }
        };

        const DIAGONALS: [IVec2; 4] = [ivec2(-1, -1), ivec2(1, -1), ivec2(-1, 1), ivec2(1, 1)];
        const CARDINALS: [IVec2; 4] = [ivec2(0, -1), ivec2(0, 1), ivec2(-1, 0), ivec2(1, 0)];

        match unit.unit_type {
            UnitType::Pawn => {
                make_moves_from_deltas(&PAWN_DELTAS);
            },
            UnitType::Knight => {
                make_moves_from_deltas(&KNIGHT_DELTAS);
            },
            UnitType::King => {
                make_moves_from_deltas(&KING_DELTAS);
            },
            UnitType::Bishop => {
                for dir in DIAGONALS {
                    let mut pos = unit.pos;
                    for _ in 0..20 {
                        pos += dir;
                        let m = Move { from: unit.pos, to: pos };
                        if self.is_valid(&m) {
                            moves.push(m);
                            if self.get_unit_at(&m.to).is_some() {
                                break;
                            }
                        }
                        else {
                            break;
                        }
                    }
                }
            },
            UnitType::Jester => {
                assert!(unit.jester_type != UnitType::Jester);
                moves = self.get_valid_moves_for_unit(&Unit { unit_type: unit.jester_type, ..*unit });
            },
            UnitType::Rook => {
                for dir in CARDINALS {
                    let mut pos = unit.pos;
                    for _ in 0..20 {
                        pos += dir;
                        let m = Move { from: unit.pos, to: pos };
                        if self.is_valid(&m) {
                            moves.push(m);
                            if self.get_unit_at(&m.to).is_some() {
                                break;
                            }
                        }
                        else {
                            break;
                        }
                    }
                }
            },
            UnitType::Queen => {
                moves.append(&mut self.get_valid_moves_for_unit(&Unit { unit_type: UnitType::Bishop, ..*unit }));
                moves.append(&mut self.get_valid_moves_for_unit(&Unit { unit_type: UnitType::Rook, ..*unit }));
            },
            UnitType::Archbishop => {
                moves.append(&mut self.get_valid_moves_for_unit(&Unit { unit_type: UnitType::Bishop, ..*unit }));
                moves.append(&mut self.get_valid_moves_for_unit(&Unit { unit_type: UnitType::Knight, ..*unit }));
            },
        }
        moves
    }

    pub fn get_valid_moves(&self, team: Team) -> Vec<Move> {
        let mut moves = vec![];
        for unit in self.units.iter().filter(|u| u.team == team) {
            moves.append(&mut self.get_valid_moves_for_unit(unit));
        }
        moves
    }

    pub fn is_on_stairs(&self) -> bool {
        if self.stairs.is_some() {
            self.units
                .iter()
                .any(|u| u.team == Team::Player && u.pos == self.stairs.unwrap())
        }
        else {
            false
        }
    }

    pub fn is_end(&self) -> bool {
        self.get_valid_moves(Team::Player).is_empty() || self.get_valid_moves(Team::Ai).is_empty()
    }

    pub fn make_move(&mut self, m: &Move) {
        // Delete unit at target position
        let mut captured_unit = None;
        if let Some(index) = self.units.iter().position(|u| u.pos == m.to) {
            captured_unit = Some(self.units[index]);
            self.units.swap_remove(index);
        }
        // Move unit to target position
        let mut unit = self.get_mut_unit_at(&m.from).unwrap();
        unit.pos = m.to;

        // Jester transformation
        if unit.unit_type == UnitType::Jester {
            if let Some(captured_unit) = captured_unit {
                unit.convert_jester(captured_unit);
            }
        }
    }
}