use std::fmt::Debug;

use crate::bishop::Bishop;
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::king::King;
use crate::knight::Knight;
use crate::pawn::Pawn;
use crate::queen::Queen;
use crate::rook::Rook;

pub trait PieceBehavior {
    fn get_move_bit_board(&self, pos: u8, occupancy_bitboard: u128, color: CurrentPlayer) -> u128;

    fn get_attack_bitboard(&self, pos: u8, occupancy_bitboard: u128, color: CurrentPlayer) -> u128;

    fn is_slider(&self) -> bool;

    fn get_all_attack_vectors(&self) -> &'static [i8] {
        &[]
    }

    fn has_opposite_vector(&self, vector: i8) -> bool {
        false
    }
}

pub fn get_all_possible_attack_vectors() -> &'static [i8] {
    &[-8, 8, 1, -1, -9, -7, 9, 7]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceType<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    None,
    Pawn(Pawn<C>),
    Rook(Rook<C>),
    Bishop(Bishop<C>),
    Knight(Knight<C>),
    Queen(Queen<C>),
    King(King<C>),
}

impl<C: BoardConfig> PieceType<C>
where
    [(); C::AREA]: Sized,
{
    pub fn value(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Pawn(_) => 100,
            Self::Queen(_) => 1200,
            Self::Rook(_) => 500,
            Self::Knight(_) => 300,
            Self::Bishop(_) => 333,
            Self::King(_) => 9999999,
        }
    }

    pub fn to_notation(&self) -> char {
        match self {
            Self::None => '.',
            Self::Pawn(_) => 'p',
            Self::Rook(_) => 'r',
            Self::Knight(_) => 'n',
            Self::Bishop(_) => 'b',
            Self::Queen(_) => 'q',
            Self::King(_) => 'k',
        }
    }

    pub fn to_index(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Pawn(_) => 1,
            Self::Rook(_) => 2,
            Self::Knight(_) => 3,
            Self::Bishop(_) => 4,
            Self::Queen(_) => 5,
            Self::King(_) => 6,
        }
    }

    pub fn get_move_bitboard(&self, at: u8, occupancy: u128, color: CurrentPlayer) -> u128 {
        match self {
            Self::None => 0,
            Self::Pawn(pawn) => pawn.get_move_bit_board(at, occupancy, color),
            Self::Rook(rook) => rook.get_move_bit_board(at, occupancy, color),
            Self::Bishop(bishop) => bishop.get_move_bit_board(at, occupancy, color),
            Self::Knight(knight) => knight.get_move_bit_board(at, occupancy, color),
            Self::Queen(queen) => queen.get_move_bit_board(at, occupancy, color),
            Self::King(king) => king.get_move_bit_board(at, occupancy, color),
        }
    }

    pub fn get_attack_bitboard(&self, at: u8, occupancy: u128, color: CurrentPlayer) -> u128 {
        match self {
            Self::None => 0,
            Self::Pawn(pawn) => pawn.get_attack_bitboard(at, occupancy, color),
            Self::Rook(rook) => rook.get_attack_bitboard(at, occupancy, color),
            Self::Bishop(bishop) => bishop.get_attack_bitboard(at, occupancy, color),
            Self::Knight(knight) => knight.get_attack_bitboard(at, occupancy, color),
            Self::Queen(queen) => queen.get_attack_bitboard(at, occupancy, color),
            Self::King(king) => king.get_attack_bitboard(at, occupancy, color),
        }
    }
    pub fn has_opposite_vector(&self, vector: i8) -> bool {
        match self {
            Self::Rook(rook) => rook.has_opposite_vector(vector),
            Self::Bishop(bishop) => bishop.has_opposite_vector(vector),
            Self::Queen(queen) => queen.has_opposite_vector(vector),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece<C: BoardConfig + Clone + Debug>
where
    [(); C::AREA]: Sized,
{
    White(PieceType<C>),
    Black(PieceType<C>),
    None,
}

impl<C: BoardConfig + Clone + Debug> Piece<C>
where
    [(); C::AREA]: Sized,
{
    pub fn get_piece(&self) -> PieceType<C> {
        match self {
            Piece::White(ptype) | Piece::Black(ptype) => ptype.clone(),
            _ => PieceType::None,
        }
    }
}
