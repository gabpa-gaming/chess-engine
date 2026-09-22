use std::marker::PhantomData;
use crate::piece::SliderMoves;
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;
use crate::bitboard::{Bitboard, BitboardIndex};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rook<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> Rook<C>
where
    [(); C::AREA]: Sized,
{
    /// Rook attack vectors: North, South, East, West
    /// North: -WIDTH (move up), South: +WIDTH (move down)
    /// East: +1 (move right), West: -1 (move left)
    pub fn get_vectors() -> [i8; 4] {
        [
            -(C::WIDTH as i8), // North
            C::WIDTH as i8,    // South
            1,                 // East
            -1,                // West
        ]
    }

    /// Static vectors for 8x8 board (standard chess)
    /// For other board sizes, use get_vectors()
    pub const STATIC_VECTORS_8X8: [i8; 4] = [-8, 8, 1, -1];
}

impl<C: BoardConfig> PieceBehavior<C> for Rook<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: C::Square, occupancy: C::Bitboard, _color: CurrentPlayer) -> C::Bitboard {
        let rank = pos.as_usize() / C::WIDTH;
        let file = pos.as_usize() % C::WIDTH;

        let mut moves = C::Bitboard::ZERO;

        let mut r = rank + 1;
        while r < C::HEIGHT {
            let idx = r * C::WIDTH + file;
            moves |= C::Bitboard::bit(C::Square::from_usize(idx));
            if occupancy.has(C::Square::from_usize(idx)) {
                break;
            }
            r += 1;
        }

        let mut r = rank as i32 - 1;
        while r >= 0 {
            let idx = (r as usize) * C::WIDTH + file;
            moves |= C::Bitboard::bit(C::Square::from_usize(idx));
            if occupancy.has(C::Square::from_usize(idx)) {
                break;
            }
            r -= 1;
        }

        let mut f = file + 1;
        while f < C::WIDTH {
            let idx = rank * C::WIDTH + f;
            moves |= C::Bitboard::bit(C::Square::from_usize(idx));
            if occupancy.has(C::Square::from_usize(idx)) {
                break;
            }
            f += 1;
        }

        let mut f = file as i32 - 1;
        while f >= 0 {
            let idx = rank * C::WIDTH + (f as usize);
            moves |= C::Bitboard::bit(C::Square::from_usize(idx));
            if occupancy.has(C::Square::from_usize(idx)) {
                break;
            }
            f -= 1;
        }

        moves
    }

    fn get_attack_bitboard(&self, pos: C::Square, occupancy: C::Bitboard, _color: CurrentPlayer) -> C::Bitboard {
        SliderMoves::<C>::slider_attacks(pos, occupancy, 0..4)
    }

    fn is_slider(&self) -> bool {
        true
    }

    fn has_opposite_vector(&self, vector: i8) -> bool {
        let vectors = Self::get_vectors();
        let neg_vector = -vector;
        vectors.contains(&neg_vector)
    }
}
