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
    pub const MOVE_BITBOARDS_BLACK: [C::Bitboard; C::AREA] =
        Self::generate_move_bitboards_black();

    pub const MOVE_BITBOARDS_WHITE: [C::Bitboard; C::AREA] =
        Self::generate_move_bitboards_white();

    pub const ATTACK_BITBOARDS_BLACK: [C::Bitboard; C::AREA] =
        Self::generate_attack_bitboards_black();

    pub const ATTACK_BITBOARDS_WHITE: [C::Bitboard; C::AREA] =
        Self::generate_attack_bitboards_white();

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

    const fn generate_move_bitboards_black()
        -> [C::Bitboard; C::AREA]
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let mut moves = C::Bitboard::ZERO;

            if rank + 1 < C::HEIGHT {
                moves = Self::add_bit(
                    moves,
                    i + C::WIDTH,
                );
            }

            if rank == 1 && rank + 2 < C::HEIGHT {
                moves = Self::add_bit(
                    moves,
                    i + 2 * C::WIDTH,
                );
            }

            bitboards[i] = moves;
            i += 1;
        }

        bitboards
    }

    const fn generate_move_bitboards_white()
        -> [C::Bitboard; C::AREA]
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let mut moves = C::Bitboard::ZERO;

            if rank > 0 {
                moves = Self::add_bit(
                    moves,
                    i - C::WIDTH,
                );
            }

            if C::HEIGHT >= 3
                && rank == C::HEIGHT - 2
                && rank >= 2
            {
                moves = Self::add_bit(
                    moves,
                    i - 2 * C::WIDTH,
                );
            }

            bitboards[i] = moves;
            i += 1;
        }

        bitboards
    }

    const fn generate_attack_bitboards_black()
        -> [C::Bitboard; C::AREA]
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            let mut attacks = C::Bitboard::ZERO;

            if rank + 1 < C::HEIGHT {
                if file > 0 {
                    attacks = Self::add_bit(
                        attacks,
                        i + C::WIDTH - 1,
                    );
                }

                if file + 1 < C::WIDTH {
                    attacks = Self::add_bit(
                        attacks,
                        i + C::WIDTH + 1,
                    );
                }
            }

            bitboards[i] = attacks;
            i += 1;
        }

        bitboards
    }

    const fn generate_attack_bitboards_white()
        -> [C::Bitboard; C::AREA]
    {
        let mut bitboards =
            [C::Bitboard::ZERO; C::AREA];

        let mut i = 0;

        while i < C::AREA {
            let rank = i / C::WIDTH;
            let file = i % C::WIDTH;

            let mut attacks = C::Bitboard::ZERO;

            if rank > 0 {
                if file > 0 {
                    attacks = Self::add_bit(
                        attacks,
                        i - C::WIDTH - 1,
                    );
                }

                if file + 1 < C::WIDTH {
                    attacks = Self::add_bit(
                        attacks,
                        i - C::WIDTH + 1,
                    );
                }
            }

            bitboards[i] = attacks;
            i += 1;
        }

        bitboards
    }
}

impl<C: BoardConfig> PieceBehavior<C> for Pawn<C>
where
    [(); C::AREA]: Sized,
{
    fn get_move_bit_board(
        &self,
        pos: C::Square,
        occupancy: C::Bitboard,
        color: CurrentPlayer,
    ) -> C::Bitboard {
        let index = pos.as_usize();
        let rank = index / C::WIDTH;
        let file = index % C::WIDTH;

        let candidates = match color {
            CurrentPlayer::White => {
                Self::MOVE_BITBOARDS_WHITE[index]
            }

            CurrentPlayer::Black => {
                Self::MOVE_BITBOARDS_BLACK[index]
            }
        };

        let next_rank = match color {
            CurrentPlayer::White => {
                if rank == 0 {
                    return C::Bitboard::ZERO;
                }

                rank - 1
            }

            CurrentPlayer::Black => {
                if rank + 1 >= C::HEIGHT {
                    return C::Bitboard::ZERO;
                }

                rank + 1
            }
        };

        let next_index =
            next_rank * C::WIDTH + file;

        let next_square =
            C::Square::from_usize(next_index);


        if occupancy.has(next_square) {
            return C::Bitboard::ZERO;
        }

        candidates & !occupancy
    }

    fn get_attack_bitboard(
        &self,
        pos: C::Square,
        _occupancy: C::Bitboard,
        color: CurrentPlayer,
    ) -> C::Bitboard {
        match color {
            CurrentPlayer::White => {
                Self::ATTACK_BITBOARDS_WHITE[pos.as_usize()]
            }

            CurrentPlayer::Black => {
                Self::ATTACK_BITBOARDS_BLACK[pos.as_usize()]
            }
        }
    }

    fn is_slider(&self) -> bool {
        false
    }
}