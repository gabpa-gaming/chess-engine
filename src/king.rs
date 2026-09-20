use std::marker::PhantomData;

use crate::bitboard::{Bitboard, BitboardIndex};
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct King<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> King<C>
where
    [(); C::AREA]: Sized,
{
    pub const MOVE_BITBOARDS: &'static [C::Bitboard] = C::KING_MOVE_BITBOARDS;
}

impl<C: BoardConfig> PieceBehavior<C> for King<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: C::Square, _occupancy: C::Bitboard, _color: CurrentPlayer) -> C::Bitboard {
        Self::MOVE_BITBOARDS[pos.as_usize()]
    }

    fn get_attack_bitboard(&self, pos: C::Square, occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        self.get_move_bit_board(pos, occupancy, color)
    }

    fn is_slider(&self) -> bool { false }
}
