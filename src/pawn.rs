use std::marker::PhantomData;

use crate::bitboard::{Bitboard, BitboardIndex};
use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Pawn<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> PieceBehavior<C> for Pawn<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: C::Square, occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        let index = pos.as_usize();
        let rank = index / C::WIDTH;
        let direction: isize = match color { CurrentPlayer::White => -1, CurrentPlayer::Black => 1 };
        let next_rank = rank as isize + direction;
        if !(0..C::HEIGHT as isize).contains(&next_rank) { return C::Bitboard::ZERO; }
        let next = next_rank as usize * C::WIDTH + index % C::WIDTH;
        let next_square = C::Square::from_usize(next);
        if occupancy.has(next_square) { return C::Bitboard::ZERO; }
        let mut moves = C::Bitboard::bit(next_square);
        let start_rank = match color { CurrentPlayer::White => C::HEIGHT - 2, CurrentPlayer::Black => 1 };
        if rank == start_rank {
            let double_rank = rank as isize + 2 * direction;
            if (0..C::HEIGHT as isize).contains(&double_rank) {
                let double_square = C::Square::from_usize(double_rank as usize * C::WIDTH + index % C::WIDTH);
                if !occupancy.has(double_square) { moves |= C::Bitboard::bit(double_square); }
            }
        }
        moves
    }

    fn get_attack_bitboard(&self, pos: C::Square, _occupancy: C::Bitboard, color: CurrentPlayer) -> C::Bitboard {
        let index = pos.as_usize();
        let rank = index / C::WIDTH;
        let file = index % C::WIDTH;
        let direction: isize = match color { CurrentPlayer::White => -1, CurrentPlayer::Black => 1 };
        let target_rank = rank as isize + direction;
        if !(0..C::HEIGHT as isize).contains(&target_rank) { return C::Bitboard::ZERO; }
        let mut attacks = C::Bitboard::ZERO;
        for target_file in [file.checked_sub(1), (file + 1 < C::WIDTH).then_some(file + 1)] {
            if let Some(target_file) = target_file {
                attacks |= C::Bitboard::bit(C::Square::from_usize(target_rank as usize * C::WIDTH + target_file));
            }
        }
        attacks
    }

    fn is_slider(&self) -> bool { false }
}
