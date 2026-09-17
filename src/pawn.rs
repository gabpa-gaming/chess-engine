use std::marker::PhantomData;

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

impl<C: BoardConfig> Pawn<C>
where
    [(); C::AREA]: Sized,
{
    pub const MOVE_BITBOARDS_BLACK: [u128; C::AREA] = Self::generate_move_bitboards_black();
    pub const MOVE_BITBOARDS_WHITE: [u128; C::AREA] = Self::generate_move_bitboards_white();
    pub const ATTACK_BITBOARDS_BLACK: [u128; C::AREA] = Self::generate_attack_bitboards_black();
    pub const ATTACK_BITBOARDS_WHITE: [u128; C::AREA] = Self::generate_attack_bitboards_white();
    const fn generate_move_bitboards_black() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        let double_step_rank = 1;
        while i < C::AREA {
            let mut moves: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            if rank + 1 < C::HEIGHT {
                let forward_idx = i + C::WIDTH;
                moves |= 1 << forward_idx;
            }
            if rank == double_step_rank {
                let double_forward_idx = 2 * C::WIDTH + i;
                moves |= 1 << double_forward_idx;
            }
            bitboards[i] = moves;
            i += 1;
        }
        bitboards
    }

    const fn generate_move_bitboards_white() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        let double_step_rank = C::HEIGHT - 2;
        while i < C::AREA {
            let mut moves: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            if rank as i32 - 1i32 >= 0 {
                let forward_idx = i - C::WIDTH;
                moves |= 1 << forward_idx;
            }
            if rank == double_step_rank {
                let double_forward_idx = i - 2 * C::WIDTH;
                moves |= 1 << double_forward_idx;
            }
            bitboards[i] = moves;
            i += 1;
        }
        bitboards
    }

    const fn generate_attack_bitboards_black() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        while i < C::AREA {
            let mut attacks: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            if rank + 1 < C::HEIGHT {
                if file > 0 {
                    attacks |= 1 << (i + C::WIDTH - 1);
                }
                if file + 1 < C::WIDTH {
                    attacks |= 1 << (i + C::WIDTH + 1);
                }
            }
            bitboards[i] = attacks;
            i += 1;
        }
        bitboards
    }

    const fn generate_attack_bitboards_white() -> [u128; C::AREA] {
        let mut bitboards = [0; C::AREA];
        let mut i = 0;

        while i < C::AREA {
            let mut attacks: u128 = 0;

            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            if rank as i32 - 1i32 >= 0 {
                if file > 0 {
                    attacks |= 1 << (i - C::WIDTH - 1);
                }
                if file + 1 < C::WIDTH {
                    attacks |= 1 << (i - C::WIDTH + 1);
                }
            }
            bitboards[i] = attacks;
            i += 1;
        }
        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior for Pawn<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(&self, pos: u8, occupancy: u128, color: CurrentPlayer) -> u128 {
        let mut push_bitboard = 0_u128;
        let width = C::WIDTH as u8;

        match color {
            CurrentPlayer::Black => {
                let single_square = pos + width;
                if ((single_square as usize) < C::AREA)
                    && ((occupancy & (1_u128 << single_square)) == 0)
                {
                    push_bitboard |= 1_u128 << single_square;

                    let row = pos / width;
                    if row == 1 {
                        let double_square = pos + (width * 2);
                        if (occupancy & (1_u128 << double_square)) == 0 {
                            push_bitboard |= 1_u128 << double_square;
                        }
                    }
                }
            }
            CurrentPlayer::White => {
                let single_square = pos.wrapping_sub(width);
                if ((single_square as usize) < C::AREA)
                    && ((occupancy & (1_u128 << single_square)) == 0)
                {
                    push_bitboard |= 1_u128 << single_square;

                    let row = pos / width;
                    let starting_row = (C::HEIGHT as u8) - 2;
                    if row == starting_row {
                        let double_square = pos.wrapping_sub(width * 2);
                        if (occupancy & (1_u128 << double_square)) == 0 {
                            push_bitboard |= 1_u128 << double_square;
                        }
                    }
                }
            }
        }

        push_bitboard
    }
    fn get_attack_bitboard(&self, pos: u8, _occupancy: u128, color: CurrentPlayer) -> u128 {
        match color {
            CurrentPlayer::White => {
                return Self::ATTACK_BITBOARDS_WHITE[pos as usize];
            }
            CurrentPlayer::Black => {
                return Self::ATTACK_BITBOARDS_BLACK[pos as usize];
            }
        }
    }

    fn is_slider(&self) -> bool {
        false
    }
}
