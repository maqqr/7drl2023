use std::default::default;

use crate::{unit::*, BoardState, boardstate::Move, utils};

pub struct Evaluation<'a> {
    pub state: BoardState<'a>,
}
impl<> Evaluation<'_> {
    pub fn from_gamestate(state: BoardState) -> Evaluation<'_> {
        Evaluation {
            state,
        }
    }

    fn shallow_clone(&self) -> Evaluation {
        Evaluation {
            state: self.state.shallow_clone(),
        }
    }

    pub fn evaluate(&self) -> f32 {
        fn unit_value(unit: &Unit) -> f32 {
            let multiplier = if unit.team == Team::Ai { -1.0 } else { 1.0 };
            multiplier * match unit.unit_type {
                UnitType::Pawn => 10.0,
                UnitType::Knight => 30.0,
                UnitType::Bishop => 30.0,
                UnitType::Jester => 40.0,
                UnitType::Rook => 50.0,
                UnitType::Archbishop => 60.0,
                UnitType::Queen => 80.0,
                UnitType::King => 100000.0,
            }
        }

        let player_close_to_stairs: f32 = if let Some(stairs) = self.state.stairs {
            self.state.units
            .iter()
            .filter(|u| u.team == Team::Player)
            .map(|u| 20.0 / (utils::dist(&stairs, &u.pos).max(1.0)))
            .sum()
        }
        else {
            0.0
        };

        // TODO closeness to player king
        let king_pos = self.state.units
            .iter()
            .find(|u| u.team == Team::Player && u.unit_type == UnitType::King)
            .map(|u| u.pos);

        let closeness =
            if let Some(king_pos) = king_pos {
                self.state.units
                    .iter()
                    .filter(|u| u.team == Team::Ai)
                    .map(|u| utils::dist(&king_pos, &u.pos))
                    .sum()
            }
            else {
                0.0
            };

        let stairs = if self.state.is_on_stairs() { 1000000.0 } else { 0.0 };

        let fake_enemy_king = unit_value(&Unit { unit_type: UnitType::King, team: Team::Ai, ..default() });
        fake_enemy_king + self.state.units.iter().map(unit_value).sum::<f32>() + player_close_to_stairs + closeness + stairs
    }

    pub fn minimax(&self, depth: u32, alpha_param: f32, beta_param: f32, maximizing_player: bool, debug: &mut Vec<(Move, f32)>) -> (Option<Move>, f32) {
        if depth == 0 || self.state.is_end() {
            let eval = self.evaluate() + if maximizing_player { 10.0 - depth as f32 } else { depth as f32 - 10.0 };
            return (None, eval)
        }

        let current_team = if maximizing_player { Team::Player } else { Team::Ai };
        let moves = self.state.get_valid_moves(current_team);
        let mut best_move: Option<Move> = moves.get(0).copied();
        let mut alpha = alpha_param;
        let mut beta = beta_param;

        if maximizing_player {
            let mut max_eval = f32::MIN;
            for r#move in moves.iter() {
                let mut eval_copy = self.shallow_clone();
                eval_copy.state.make_move(r#move);
                let current_eval = eval_copy.minimax(depth - 1, alpha, beta, false, &mut vec![]).1;

                if current_eval > max_eval {
                    max_eval = current_eval;
                    best_move = Some(*r#move);
                }
                if max_eval >= beta {
                    break
                }
                if max_eval > alpha {
                    alpha = max_eval
                }
                debug.push((*r#move, current_eval));
            }
            (best_move, max_eval)
        }
        else {
            let mut min_eval = f32::MAX;
            for r#move in moves.iter() {
                let mut eval_copy = self.shallow_clone();
                eval_copy.state.make_move(r#move);
                let current_eval = eval_copy.minimax(depth - 1, alpha, beta, true, &mut vec![]).1;

                if current_eval < min_eval {
                    min_eval = current_eval;
                    best_move = Some(*r#move);
                }
                if min_eval <= alpha {
                    break
                }
                if min_eval < beta {
                    beta = min_eval;
                }
                debug.push((*r#move, current_eval));
            }
            (best_move, min_eval)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, default::default};
    use macroquad::prelude::ivec2;
    use super::*;

    #[test]
    fn test() {
        use crate::tile::TileMap;
        let map_plan = [
        "..#####",
        "##..##.",
        "##...#.",
        ".......",
        "##...##",
        "##...#.",
        "##...#."];

        let mut gamestate = BoardState {
            tilemap: Cow::Owned(TileMap::from(&map_plan[..])),
            ..default()
        };
        gamestate.units.push(Unit { pos: ivec2(3, 2), unit_type: UnitType::Pawn, team: Team::Player, ..default() });
        gamestate.units.push(Unit { pos: ivec2(2, 3), unit_type: UnitType::Knight, team: Team::Ai, ..default() });
        gamestate.units.push(Unit { pos: ivec2(3, 3), unit_type: UnitType::Knight, team: Team::Ai, ..default() });

        let eval = Evaluation::from_gamestate(gamestate);

        let mut eval2 = eval.shallow_clone();
        eval2.state.make_move(&Move { from: ivec2(3, 2), to: ivec2(3, 3) });

        assert_eq!(eval.state.units.get(0).unwrap().pos, ivec2(3, 2));
        assert_eq!(eval2.state.units.get(0).unwrap().pos, ivec2(3, 3));

        assert_eq!(eval.state.units.len(), 3);
        assert_eq!(eval2.state.units.len(), 2);
    }
}
