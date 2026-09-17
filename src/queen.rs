use std::marker::PhantomData;

use crate::board_config::BoardConfig;
use crate::chess_board::CurrentPlayer;
use crate::piece::PieceBehavior;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Queen<C: BoardConfig>
where
    [(); C::AREA]: Sized,
{
    _marker: PhantomData<C>,
}

impl<C: BoardConfig> Queen<C>
where
    [(); C::AREA]: Sized,
{
    /// Queen attack vectors: 8 directions (4 cardinal + 4 diagonal)
    /// Cardinal: North, South, East, West
    /// Diagonal: Northeast, Northwest, Southeast, Southwest
    pub fn get_vectors() -> [i8; 8] {
        let w = C::WIDTH as i8;
        [
            -w,       // North
            w,        // South
            1,        // East
            -1,       // West
            -(w + 1), // Northeast
            -(w - 1), // Northwest
            w + 1,    // Southeast
            w - 1,    // Southwest
        ]
    }
}

impl<C: BoardConfig> PieceBehavior for Queen<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: u8, occupancy: u128, color_: CurrentPlayer) -> u128 {
        let rank = pos as usize / C::WIDTH;
        let file = pos as usize % C::WIDTH;

        let mut moves: u128 = 0;

        let mut r = rank + 1;
        while r < C::HEIGHT {
            let idx = r * C::WIDTH + file;
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r += 1;
        }

        let mut r = rank as i32 - 1;
        while r >= 0 {
            let idx = (r as usize) * C::WIDTH + file;
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            r -= 1;
        }

        let mut f = file + 1;
        while f < C::WIDTH {
            let idx = rank * C::WIDTH + f;
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            f += 1;
        }

        let mut f = file as i32 - 1;
        while f >= 0 {
            let idx = rank * C::WIDTH + (f as usize);
            moves |= 1 << idx;
            if (occupancy >> idx) & 1 == 1 {
                break;
            }
            f -= 1;
        }

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

    fn get_attack_bitboard(&self, pos: u8, occupancy: u128, _color: CurrentPlayer) -> u128 {
        self.get_move_bit_board(pos, occupancy, _color)
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
