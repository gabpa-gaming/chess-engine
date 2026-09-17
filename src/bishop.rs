use std::marker::PhantomData;

use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Bishop<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> Bishop<C>
where
    [(); C::AREA]: Sized,
{
    /// Bishop attack vectors: Northeast, Northwest, Southeast, Southwest
    /// Northeast: -WIDTH-1, Northwest: -WIDTH+1, Southeast: +WIDTH+1, Southwest: +WIDTH-1
    pub fn get_vectors() -> [i8; 4] {
        let w = C::WIDTH as i8;
        [
            -(w + 1), // Northeast
            -(w - 1), // Northwest
            w + 1,    // Southeast
            w - 1,    // Southwest
        ]
    }
}

impl<C: BoardConfig> PieceBehavior for Bishop<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: u8, occupancy: u128, _color: CurrentPlayer) -> u128 {
        let rank = pos as usize / C::WIDTH;
        let file = pos as usize % C::WIDTH;

        let mut moves: u128 = 0;

        let mut r = rank + 1;
        let mut f = file + 1;
        while r < C::HEIGHT && f < C::WIDTH {
            let idx = r * C::WIDTH + f;
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r += 1;
            f += 1;
        }

        let mut r = rank + 1;
        let mut f = file as i32 - 1;
        while r < C::HEIGHT && f >= 0 {
            let idx = r * C::WIDTH + (f as usize);
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r += 1;
            f -= 1;
        }

        let mut r = rank as i32 - 1;
        let mut f = file + 1;
        while r >= 0 && f < C::WIDTH {
            let idx = (r as usize) * C::WIDTH + f;
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r -= 1;
            f += 1;
        }

        let mut r = rank as i32 - 1;
        let mut f = file as i32 - 1;
        while r >= 0 && f >= 0 {
            let idx = (r as usize) * C::WIDTH + (f as usize);
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r -= 1;
            f -= 1;
        }

        moves
    }

    fn get_attack_bitboard(&self, pos: u8, occupancy: u128, color: CurrentPlayer) -> u128 {
        self.get_move_bit_board(pos, occupancy, color)
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
