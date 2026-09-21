use std::marker::PhantomData;

use crate::bitboard::{
    Bitboard,
    BitboardIndex,
    ConstBitboardOps,
};
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

impl<C: BoardConfig> Knight<C>
where
    [(); C::AREA]: Sized,
{
    pub const MOVE_BITBOARDS: [C::Bitboard; C::AREA] =
        Self::generate_move_bitboards();

    const fn add_bit(
        board: C::Bitboard,
        index: usize,
    ) -> C::Bitboard 
    where
        C::Bitboard: [const] ConstBitboardOps,
    {
        let bit =
            <C::Bitboard as ConstBitboardOps>::const_bit(index);
    
        <C::Bitboard as ConstBitboardOps>::const_or(board, bit)
    }

    const fn generate_move_bitboards()
        -> [C::Bitboard; C::AREA]
        where
                C::Bitboard: const ConstBitboardOps,
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let offsets: [(i32, i32); 8] = [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            let mut moves = C::Bitboard::ZERO;
            let mut j = 0;

            while j < offsets.len() {
                let rank_offset = offsets[j].0;
                let file_offset = offsets[j].1;

                let new_rank =
                    rank as i32 + rank_offset;

                let new_file =
                    file as i32 + file_offset;

                if new_rank >= 0
                    && new_rank < C::HEIGHT as i32
                    && new_file >= 0
                    && new_file < C::WIDTH as i32
                {
                    let target =
                        new_rank as usize * C::WIDTH
                            + new_file as usize;

                    moves = Self::add_bit(
                        moves,
                        target,
                    );
                }

                j += 1;
            }

            bitboards[i] = moves;
            i += 1;
        }

        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior<C> for Knight<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(
        &self,
        pos: C::Square,
        _occupancy: C::Bitboard,
        _color: CurrentPlayer,
    ) -> C::Bitboard {
        Self::MOVE_BITBOARDS[pos.as_usize()]
    }

    fn get_attack_bitboard(
        &self,
        pos: C::Square,
        _occupancy: C::Bitboard,
        _color: CurrentPlayer,
    ) -> C::Bitboard {
        Self::MOVE_BITBOARDS[pos.as_usize()]
    }

    fn is_slider(&self) -> bool {
        false
    }
}