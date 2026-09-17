use std::marker::PhantomData;

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
    pub const MOVE_BITBOARDS: [u128; C::AREA] = Self::generate_move_bitboards();

    const fn generate_move_bitboards() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        while i < C::AREA {
            let mut moves: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            if rank + 1 < C::HEIGHT {
                let up_idx = i + C::WIDTH;
                moves |= 1 << up_idx; // N

                if file > 0 {
                    moves |= 1 << (up_idx - 1);
                } // NW
                if file + 1 < C::WIDTH {
                    moves |= 1 << (up_idx + 1);
                } // NE
            }

            // SOUTH (Rank - 1)
            if rank > 0 {
                let down_idx = i - C::WIDTH;
                moves |= 1 << down_idx; // S

                if file > 0 {
                    moves |= 1 << (down_idx - 1);
                } // SW
                if file + 1 < C::WIDTH {
                    moves |= 1 << (down_idx + 1);
                } // SE
            }

            if file > 0 {
                moves |= 1 << (i - 1);
            } // W
            if file + 1 < C::WIDTH {
                moves |= 1 << (i + 1);
            } // E

            bitboards[i] = moves;
            i += 1;
        }
        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior for King<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: u8, _occupancy: u128, color: CurrentPlayer) -> u128 {
        Self::MOVE_BITBOARDS[pos as usize]
    }

    fn get_attack_bitboard(&self, pos: u8, _occupancy: u128, _color: CurrentPlayer) -> u128 {
        Self::MOVE_BITBOARDS[pos as usize]
    }

    fn is_slider(&self) -> bool {
        false
    }
}
