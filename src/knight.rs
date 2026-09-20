use std::marker::PhantomData;

use crate::bitboard::{Bitboard, BitboardIndex};
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Knight<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> PieceBehavior<C> for Knight<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: C::Square, _occupancy: C::Bitboard, _color: CurrentPlayer) -> C::Bitboard {
        let rank = pos.as_usize() / C::WIDTH;
        let file = pos.as_usize() % C::WIDTH;
        let mut moves = C::Bitboard::ZERO;
        for (rank_offset, file_offset) in [(-2_i32, -1_i32), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)] {
            let target_rank = rank as i32 + rank_offset;
            let target_file = file as i32 + file_offset;
            if target_rank >= 0 && target_rank < C::HEIGHT as i32 && target_file >= 0 && target_file < C::WIDTH as i32 {
                moves |= C::Bitboard::bit(C::Square::from_usize(target_rank as usize * C::WIDTH + target_file as usize));
            }
        }
        moves
    }

    fn get_attack_bitboard(&self, pos: C::Square, occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        self.get_move_bit_board(pos, occupancy, color)
    }

    fn is_slider(&self) -> bool { false }
}
