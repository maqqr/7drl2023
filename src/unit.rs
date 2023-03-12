use macroquad::prelude::IVec2;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    #[default] Player,
    Ai,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnitType {
    #[default] Pawn,
    Knight,
    King,
    Bishop,
    Jester,
    Rook,
    Queen,
    Archbishop,
}

#[derive(Default, Clone, Copy, Hash)]
pub struct Unit {
    pub pos: IVec2,
    pub unit_type: UnitType,
    pub jester_type: UnitType,
    pub team: Team,
}

impl Unit {
    pub fn convert_jester(&mut self, captured_unit: Unit) {
        self.jester_type =
            if captured_unit.unit_type == UnitType::Jester {
                captured_unit.jester_type
            }
            else {
                captured_unit.unit_type
            };
        assert!(self.jester_type != UnitType::Jester);
    }
}

pub fn is_enemy(unit: &Unit, other_unit: &Unit) -> bool {
    unit.team != other_unit.team
}

pub fn material_reward(unit_type: UnitType) -> i32 {
    match unit_type {
        UnitType::Pawn => 1,
        UnitType::Knight => 3,
        UnitType::Bishop => 3,
        UnitType::Jester => 4,
        UnitType::Rook => 6,
        UnitType::Archbishop => 6,
        UnitType::Queen => 6,
        UnitType::King => 100,
    }
}

pub fn unit_buy_price(unit_type: UnitType) -> i32 {
    match unit_type {
        UnitType::Pawn => 1,
        UnitType::Knight => 3,
        UnitType::Bishop => 3,
        UnitType::Jester => 4,
        UnitType::Rook => 6,
        UnitType::Archbishop => 7,
        UnitType::Queen => 9,
        UnitType::King => 100,
    }
}

pub fn unit_description(unit_type: UnitType) -> &'static str {
    match unit_type {
        UnitType::Pawn => "Moves one square in any direction, but not diagonally.",
        UnitType::Knight => "Moves in L shape, can jump over pieces.",
        UnitType::Bishop => "Moves any amount diagonally.",
        UnitType::Jester => "Like a Rook, until it takes move style from captured piece.",
        UnitType::Rook => "Moves any amount up, down, left or right.",
        UnitType::Queen => "Moves like a Bishop and a Rook combined.",
        UnitType::King => "Moves one square in any direction.",
        UnitType::Archbishop => "Moves like a Knight and a Bishop combined.",
    }
}