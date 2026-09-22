use std::fmt::Debug;
use std::marker::PhantomData;
use crate::bitboard::BitboardIndex;
use crate::bitboard::ConstBitboardOps;
use crate::bishop::Bishop;
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::king::King;
use crate::knight::Knight;
use crate::pawn::Pawn;
use crate::queen::Queen;
use crate::rook::Rook;
use crate::bitboard::Bitboard;

pub trait PieceBehavior<C: BoardConfig> {
    fn get_move_bit_board(&self, pos: C::Square, occupancy_bitboard: C::Bitboard, color: CurrentPlayer) -> C::Bitboard;

    fn get_attack_bitboard(&self, pos: C::Square, occupancy_bitboard: C::Bitboard, color: CurrentPlayer) -> C::Bitboard;

    fn is_slider(&self) -> bool;

    fn get_all_attack_vectors(&self) -> &'static [i8] {
        &[]
    }

    fn has_opposite_vector(&self, _vector: i8) -> bool {
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

    pub fn get_move_bitboard(&self, at: C::Square, occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        match self {
            Self::None => C::Bitboard::ZERO,
            Self::Pawn(pawn) => pawn.get_move_bit_board(at, occupancy, color),
            Self::Rook(rook) => rook.get_move_bit_board(at, occupancy, color),
            Self::Bishop(bishop) => bishop.get_move_bit_board(at, occupancy, color),
            Self::Knight(knight) => knight.get_move_bit_board(at, occupancy, color),
            Self::Queen(queen) => queen.get_move_bit_board(at, occupancy, color),
            Self::King(king) => king.get_move_bit_board(at, occupancy, color),
        }
    }

    pub fn get_attack_bitboard(&self, at: C::Square, occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        match self {
            Self::None => C::Bitboard::ZERO,
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

pub struct SliderMoves<C: BoardConfig> {
    _phantom_data: PhantomData<C>
}

impl<C: BoardConfig> SliderMoves<C>
where 
    [(); C::AREA]: Sized,
{
    const INCREASING: [bool; 8] = [
            true, false, true, false,
            true, true, false, false,
        ];
        
    const DIRECTIONS: [(i32, i32); 8] = [
        ( 1,  0),
        (-1,  0),
        ( 0,  1),
        ( 0, -1),
        ( 1,  1),
        ( 1, -1),
        (-1,  1),
        (-1, -1),
    ];
    
    pub const RAYS: [[C::Bitboard; 8]; C::AREA] = Self::generate_rays();

    const fn generate_rays() -> [[C::Bitboard; 8]; C::AREA]
    where
        C::Bitboard: const ConstBitboardOps,
    {
        let mut rays = [[C::Bitboard::ZERO; 8]; C::AREA];
        let mut square = 0;

        while square < C::AREA {
            let rank = (square / C::WIDTH) as i32;
            let file = (square % C::WIDTH) as i32;

            let mut direction = 0;
            while direction < 8 {
                let (dr, df) = Self::DIRECTIONS[direction];
                let mut r = rank + dr;
                let mut f = file + df;

                while r >= 0
                    && r < C::HEIGHT as i32
                    && f >= 0
                    && f < C::WIDTH as i32
                {
                    let index = r as usize * C::WIDTH + f as usize;
                    let bit =
                        <C::Bitboard as ConstBitboardOps>::const_bit(index);

                    rays[square][direction] =
                        <C::Bitboard as ConstBitboardOps>::const_or(
                            rays[square][direction],
                            bit,
                        );

                    r += dr;
                    f += df;
                }

                direction += 1;
            }

            square += 1;
        }

        rays
    }
    
    pub fn slider_attacks(
        pos: C::Square,
        occupancy: C::Bitboard,
        directions: std::ops::Range<usize>,
    ) -> C::Bitboard {
        let mut attacks = C::Bitboard::ZERO;
    
        for direction in directions {
            let mut ray = Self::RAYS[pos.as_usize()][direction];
            let blockers = ray & occupancy;
    
            if blockers.any() {
                let nearest = if Self::INCREASING[direction] {
                    blockers.first_set().unwrap()
                } else {
                    blockers.last_set().unwrap()
                };
    
                ray &= !Self::RAYS[nearest.as_usize()][direction];
            }
    
            attacks |= ray;
        }
    
        attacks
    }
}
