use std::marker::PhantomData;

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
    pub const MOVE_BITBOARDS: [u128; C::AREA] = Self::generate_move_bitboards();

    const fn generate_move_bitboards() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        while i < C::AREA {
            let mut moves: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            // All 8 possible knight moves
            let offsets = [
                (-2, -1),
                (-2, 1),
                (-1, -2),
                (-1, 2),
                (1, -2),
                (1, 2),
                (2, -1),
                (2, 1),
            ];

            let mut j = 0;
            while j < offsets.len() {
                let (rank_offset, file_offset) = offsets[j];
                let new_rank = rank as i32 + rank_offset;
                let new_file = file as i32 + file_offset;

                if new_rank >= 0
                    && new_rank < C::HEIGHT as i32
                    && new_file >= 0
                    && new_file < C::WIDTH as i32
                {
                    let idx = (new_rank as usize * C::WIDTH) + new_file as usize;
                    moves |= 1 << idx;
                }

                j += 1;
            }

            bitboards[i] = moves;
            i += 1;
        }
        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior for Knight<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: u8, _occupancy: u128, color_: CurrentPlayer) -> u128 {
        Self::MOVE_BITBOARDS[pos as usize]
    }

    fn get_attack_bitboard(&self, pos: u8, _occupancy: u128, _color: CurrentPlayer) -> u128 {
        Self::MOVE_BITBOARDS[pos as usize]
    }

    fn is_slider(&self) -> bool {
        false
    }
}
