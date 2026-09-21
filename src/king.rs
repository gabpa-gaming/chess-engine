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
    pub const MOVE_BITBOARDS: [C::Bitboard; C::AREA] =
        Self::generate_move_bitboards();

    const fn add_bit(
        bitboard: C::Bitboard,
        index: usize,
    ) -> C::Bitboard {
        let bit =
            <C::Bitboard as ConstBitboardOps>::const_bit(index);

        <C::Bitboard as ConstBitboardOps>::const_or(
            bitboard,
            bit,
        )
    }

    const fn generate_move_bitboards()
        -> [C::Bitboard; C::AREA]
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            let mut moves = C::Bitboard::ZERO;

            // North
            if rank + 1 < C::HEIGHT {
                let up = i + C::WIDTH;

                moves = Self::add_bit(moves, up);

                // North-west
                if file > 0 {
                    moves = Self::add_bit(
                        moves,
                        up - 1,
                    );
                }

                // North-east
                if file + 1 < C::WIDTH {
                    moves = Self::add_bit(
                        moves,
                        up + 1,
                    );
                }
            }

            // South
            if rank > 0 {
                let down = i - C::WIDTH;

                moves = Self::add_bit(moves, down);

                // South-west
                if file > 0 {
                    moves = Self::add_bit(
                        moves,
                        down - 1,
                    );
                }

                // South-east
                if file + 1 < C::WIDTH {
                    moves = Self::add_bit(
                        moves,
                        down + 1,
                    );
                }
            }

            // West
            if file > 0 {
                moves = Self::add_bit(
                    moves,
                    i - 1,
                );
            }

            // East
            if file + 1 < C::WIDTH {
                moves = Self::add_bit(
                    moves,
                    i + 1,
                );
            }

            bitboards[i] = moves;
            i += 1;
        }

        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior<C> for King<C>
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