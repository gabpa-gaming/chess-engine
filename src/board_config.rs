use crate::bitboard::{Bitboard, BitboardIndex, MultiBitboard, U16384, U512};
use crate::chess_move::{BigMove, MoveTrait, RegularMove};
use std::fmt::Debug;

const fn king_moves_u64<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>() -> [u64; AREA] {
    let mut bitboards = [0; AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut moves = 0;
        let mut rank_offset = -1_i32;
        while rank_offset <= 1 {
            let mut file_offset = -1_i32;
            while file_offset <= 1 {
                if rank_offset != 0 || file_offset != 0 {
                    let target_rank = rank as i32 + rank_offset;
                    let target_file = file as i32 + file_offset;
                    if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                        moves |= 1 << (target_rank as usize * WIDTH + target_file as usize);
                    }
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        bitboards[square] = moves;
        square += 1;
    }
    bitboards
}

const fn knight_moves_u64<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>() -> [u64; AREA] {
    let mut bitboards = [0; AREA];
    let offsets = [(-2_i32, -1_i32), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut moves = 0;
        let mut offset = 0;
        while offset < offsets.len() {
            let target_rank = rank as i32 + offsets[offset].0;
            let target_file = file as i32 + offsets[offset].1;
            if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                moves |= 1 << (target_rank as usize * WIDTH + target_file as usize);
            }
            offset += 1;
        }
        bitboards[square] = moves;
        square += 1;
    }
    bitboards
}

const fn king_moves_u128<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>() -> [u128; AREA] {
    let mut bitboards = [0; AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut moves = 0;
        let mut rank_offset = -1_i32;
        while rank_offset <= 1 {
            let mut file_offset = -1_i32;
            while file_offset <= 1 {
                if rank_offset != 0 || file_offset != 0 {
                    let target_rank = rank as i32 + rank_offset;
                    let target_file = file as i32 + file_offset;
                    if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                        moves |= 1 << (target_rank as usize * WIDTH + target_file as usize);
                    }
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        bitboards[square] = moves;
        square += 1;
    }
    bitboards
}

const fn knight_moves_u128<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>() -> [u128; AREA] {
    let mut bitboards = [0; AREA];
    let offsets = [(-2_i32, -1_i32), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut moves = 0;
        let mut offset = 0;
        while offset < offsets.len() {
            let target_rank = rank as i32 + offsets[offset].0;
            let target_file = file as i32 + offsets[offset].1;
            if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                moves |= 1 << (target_rank as usize * WIDTH + target_file as usize);
            }
            offset += 1;
        }
        bitboards[square] = moves;
        square += 1;
    }
    bitboards
}

const fn king_moves_multi<const WIDTH: usize, const HEIGHT: usize, const AREA: usize, const WORDS: usize>() -> [MultiBitboard<WORDS>; AREA] {
    let mut bitboards = [MultiBitboard([0; WORDS]); AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut rank_offset = -1_i32;
        while rank_offset <= 1 {
            let mut file_offset = -1_i32;
            while file_offset <= 1 {
                if rank_offset != 0 || file_offset != 0 {
                    let target_rank = rank as i32 + rank_offset;
                    let target_file = file as i32 + file_offset;
                    if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                        let target = target_rank as usize * WIDTH + target_file as usize;
                        bitboards[square].0[target / 64] |= 1 << (target % 64);
                    }
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        square += 1;
    }
    bitboards
}

const fn knight_moves_multi<const WIDTH: usize, const HEIGHT: usize, const AREA: usize, const WORDS: usize>() -> [MultiBitboard<WORDS>; AREA] {
    let mut bitboards = [MultiBitboard([0; WORDS]); AREA];
    let offsets = [(-2_i32, -1_i32), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut offset = 0;
        while offset < offsets.len() {
            let target_rank = rank as i32 + offsets[offset].0;
            let target_file = file as i32 + offsets[offset].1;
            if target_rank >= 0 && target_rank < HEIGHT as i32 && target_file >= 0 && target_file < WIDTH as i32 {
                let target = target_rank as usize * WIDTH + target_file as usize;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
            offset += 1;
        }
        square += 1;
    }
    bitboards
}

const fn pawn_bitboards_u64<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>(mode: u8) -> [u64; AREA] {
    let mut bitboards = [0; AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut bits = 0;
        if mode == 0 && rank + 1 < HEIGHT {
            bits |= 1 << (square + WIDTH);
            if rank == 1 { bits |= 1 << (square + WIDTH * 2); }
        } else if mode == 1 && rank > 0 {
            bits |= 1 << (square - WIDTH);
            if rank == HEIGHT - 2 { bits |= 1 << (square - WIDTH * 2); }
        } else if mode == 2 && rank + 1 < HEIGHT {
            if file > 0 { bits |= 1 << (square + WIDTH - 1); }
            if file + 1 < WIDTH { bits |= 1 << (square + WIDTH + 1); }
        } else if mode == 3 && rank > 0 {
            if file > 0 { bits |= 1 << (square - WIDTH - 1); }
            if file + 1 < WIDTH { bits |= 1 << (square - WIDTH + 1); }
        }
        bitboards[square] = bits;
        square += 1;
    }
    bitboards
}

const fn pawn_bitboards_u128<const WIDTH: usize, const HEIGHT: usize, const AREA: usize>(mode: u8) -> [u128; AREA] {
    let mut bitboards = [0; AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        let mut bits = 0;
        if mode == 0 && rank + 1 < HEIGHT {
            bits |= 1 << (square + WIDTH);
            if rank == 1 { bits |= 1 << (square + WIDTH * 2); }
        } else if mode == 1 && rank > 0 {
            bits |= 1 << (square - WIDTH);
            if rank == HEIGHT - 2 { bits |= 1 << (square - WIDTH * 2); }
        } else if mode == 2 && rank + 1 < HEIGHT {
            if file > 0 { bits |= 1 << (square + WIDTH - 1); }
            if file + 1 < WIDTH { bits |= 1 << (square + WIDTH + 1); }
        } else if mode == 3 && rank > 0 {
            if file > 0 { bits |= 1 << (square - WIDTH - 1); }
            if file + 1 < WIDTH { bits |= 1 << (square - WIDTH + 1); }
        }
        bitboards[square] = bits;
        square += 1;
    }
    bitboards
}

const fn pawn_bitboards_multi<const WIDTH: usize, const HEIGHT: usize, const AREA: usize, const WORDS: usize>(mode: u8) -> [MultiBitboard<WORDS>; AREA] {
    let mut bitboards = [MultiBitboard([0; WORDS]); AREA];
    let mut square = 0;
    while square < AREA {
        let rank = square / WIDTH;
        let file = square % WIDTH;
        if mode == 0 && rank + 1 < HEIGHT {
            let target = square + WIDTH;
            bitboards[square].0[target / 64] |= 1 << (target % 64);
            if rank == 1 {
                let target = square + WIDTH * 2;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
        } else if mode == 1 && rank > 0 {
            let target = square - WIDTH;
            bitboards[square].0[target / 64] |= 1 << (target % 64);
            if rank == HEIGHT - 2 {
                let target = square - WIDTH * 2;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
        } else if mode == 2 && rank + 1 < HEIGHT {
            if file > 0 {
                let target = square + WIDTH - 1;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
            if file + 1 < WIDTH {
                let target = square + WIDTH + 1;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
        } else if mode == 3 && rank > 0 {
            if file > 0 {
                let target = square - WIDTH - 1;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
            if file + 1 < WIDTH {
                let target = square - WIDTH + 1;
                bitboards[square].0[target / 64] |= 1 << (target % 64);
            }
        }
        square += 1;
    }
    bitboards
}

static REGULAR_KING_MOVE_BITBOARDS: [u64; 64] = king_moves_u64::<8, 8, 64>();
static REGULAR_KNIGHT_MOVE_BITBOARDS: [u64; 64] = knight_moves_u64::<8, 8, 64>();
static BIG_KING_MOVE_BITBOARDS: [u128; 120] = king_moves_u128::<10, 12, 120>();
static BIG_KNIGHT_MOVE_BITBOARDS: [u128; 120] = knight_moves_u128::<10, 12, 120>();
static HUGE_KING_MOVE_BITBOARDS: [U512; 400] = king_moves_multi::<20, 20, 400, 8>();
static HUGE_KNIGHT_MOVE_BITBOARDS: [U512; 400] = knight_moves_multi::<20, 20, 400, 8>();
static HUNDRED_KING_MOVE_BITBOARDS: [U16384; 10_000] = king_moves_multi::<100, 100, 10_000, 256>();
static HUNDRED_KNIGHT_MOVE_BITBOARDS: [U16384; 10_000] = knight_moves_multi::<100, 100, 10_000, 256>();
static REGULAR_PAWN_BLACK_MOVE_BITBOARDS: [u64; 64] = pawn_bitboards_u64::<8, 8, 64>(0);
static REGULAR_PAWN_WHITE_MOVE_BITBOARDS: [u64; 64] = pawn_bitboards_u64::<8, 8, 64>(1);
static REGULAR_PAWN_BLACK_ATTACK_BITBOARDS: [u64; 64] = pawn_bitboards_u64::<8, 8, 64>(2);
static REGULAR_PAWN_WHITE_ATTACK_BITBOARDS: [u64; 64] = pawn_bitboards_u64::<8, 8, 64>(3);
static BIG_PAWN_BLACK_MOVE_BITBOARDS: [u128; 120] = pawn_bitboards_u128::<10, 12, 120>(0);
static BIG_PAWN_WHITE_MOVE_BITBOARDS: [u128; 120] = pawn_bitboards_u128::<10, 12, 120>(1);
static BIG_PAWN_BLACK_ATTACK_BITBOARDS: [u128; 120] = pawn_bitboards_u128::<10, 12, 120>(2);
static BIG_PAWN_WHITE_ATTACK_BITBOARDS: [u128; 120] = pawn_bitboards_u128::<10, 12, 120>(3);
static HUGE_PAWN_BLACK_MOVE_BITBOARDS: [U512; 400] = pawn_bitboards_multi::<20, 20, 400, 8>(0);
static HUGE_PAWN_WHITE_MOVE_BITBOARDS: [U512; 400] = pawn_bitboards_multi::<20, 20, 400, 8>(1);
static HUGE_PAWN_BLACK_ATTACK_BITBOARDS: [U512; 400] = pawn_bitboards_multi::<20, 20, 400, 8>(2);
static HUGE_PAWN_WHITE_ATTACK_BITBOARDS: [U512; 400] = pawn_bitboards_multi::<20, 20, 400, 8>(3);
static HUNDRED_PAWN_BLACK_MOVE_BITBOARDS: [U16384; 10_000] = pawn_bitboards_multi::<100, 100, 10_000, 256>(0);
static HUNDRED_PAWN_WHITE_MOVE_BITBOARDS: [U16384; 10_000] = pawn_bitboards_multi::<100, 100, 10_000, 256>(1);
static HUNDRED_PAWN_BLACK_ATTACK_BITBOARDS: [U16384; 10_000] = pawn_bitboards_multi::<100, 100, 10_000, 256>(2);
static HUNDRED_PAWN_WHITE_ATTACK_BITBOARDS: [U16384; 10_000] = pawn_bitboards_multi::<100, 100, 10_000, 256>(3);

pub trait BoardConfig: Sized + Clone + Default + Debug + Send + Sync {
    const WIDTH: usize;
    const HEIGHT: usize;
    const AREA: usize = Self::WIDTH * Self::HEIGHT;
    type Square: BitboardIndex;
    type Bitboard: Bitboard<Index = Self::Square> + Send + Sync;
    const KING_MOVE_BITBOARDS: &'static [Self::Bitboard];
    const KNIGHT_MOVE_BITBOARDS: &'static [Self::Bitboard];
    const PAWN_BLACK_MOVE_BITBOARDS: &'static [Self::Bitboard];
    const PAWN_WHITE_MOVE_BITBOARDS: &'static [Self::Bitboard];
    const PAWN_BLACK_ATTACK_BITBOARDS: &'static [Self::Bitboard];
    const PAWN_WHITE_ATTACK_BITBOARDS: &'static [Self::Bitboard];
    type MoveType: MoveTrait<Self> + Debug + Copy + Clone + PartialEq + Eq
    where
        [(); Self::AREA]: Sized;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegularVariant;

impl Default for RegularVariant {
    fn default() -> Self { Self }
}

impl BoardConfig for RegularVariant {
    const WIDTH: usize = 8;
    const HEIGHT: usize = 8;
    type MoveType = RegularMove;
    type Square = u8;
    type Bitboard = u64;
    const KING_MOVE_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_KING_MOVE_BITBOARDS;
    const KNIGHT_MOVE_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_KNIGHT_MOVE_BITBOARDS;
    const PAWN_BLACK_MOVE_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_PAWN_BLACK_MOVE_BITBOARDS;
    const PAWN_WHITE_MOVE_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_PAWN_WHITE_MOVE_BITBOARDS;
    const PAWN_BLACK_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_PAWN_BLACK_ATTACK_BITBOARDS;
    const PAWN_WHITE_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &REGULAR_PAWN_WHITE_ATTACK_BITBOARDS;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BigVariant;

impl Default for BigVariant {
    fn default() -> Self { Self }
}

impl BoardConfig for BigVariant {
    const WIDTH: usize = 10;
    const HEIGHT: usize = 12;
    type MoveType = BigMove<Self>;
    type Square = u8;
    type Bitboard = u128;
    const KING_MOVE_BITBOARDS: &'static [Self::Bitboard] = &BIG_KING_MOVE_BITBOARDS;
    const KNIGHT_MOVE_BITBOARDS: &'static [Self::Bitboard] = &BIG_KNIGHT_MOVE_BITBOARDS;
    const PAWN_BLACK_MOVE_BITBOARDS: &'static [Self::Bitboard] = &BIG_PAWN_BLACK_MOVE_BITBOARDS;
    const PAWN_WHITE_MOVE_BITBOARDS: &'static [Self::Bitboard] = &BIG_PAWN_WHITE_MOVE_BITBOARDS;
    const PAWN_BLACK_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &BIG_PAWN_BLACK_ATTACK_BITBOARDS;
    const PAWN_WHITE_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &BIG_PAWN_WHITE_ATTACK_BITBOARDS;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HugeVariant;

impl Default for HugeVariant {
    fn default() -> Self { Self }
}

impl BoardConfig for HugeVariant {
    const WIDTH: usize = 20;
    const HEIGHT: usize = 20;
    type MoveType = BigMove<Self>;
    type Square = u16;
    type Bitboard = U512;
    const KING_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUGE_KING_MOVE_BITBOARDS;
    const KNIGHT_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUGE_KNIGHT_MOVE_BITBOARDS;
    const PAWN_BLACK_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUGE_PAWN_BLACK_MOVE_BITBOARDS;
    const PAWN_WHITE_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUGE_PAWN_WHITE_MOVE_BITBOARDS;
    const PAWN_BLACK_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &HUGE_PAWN_BLACK_ATTACK_BITBOARDS;
    const PAWN_WHITE_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &HUGE_PAWN_WHITE_ATTACK_BITBOARDS;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HundredVariant;

impl Default for HundredVariant {
    fn default() -> Self { Self }
}

impl BoardConfig for HundredVariant {
    const WIDTH: usize = 100;
    const HEIGHT: usize = 100;
    type MoveType = BigMove<Self>;
    type Square = u16;
    type Bitboard = U16384;
    const KING_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_KING_MOVE_BITBOARDS;
    const KNIGHT_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_KNIGHT_MOVE_BITBOARDS;
    const PAWN_BLACK_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_PAWN_BLACK_MOVE_BITBOARDS;
    const PAWN_WHITE_MOVE_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_PAWN_WHITE_MOVE_BITBOARDS;
    const PAWN_BLACK_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_PAWN_BLACK_ATTACK_BITBOARDS;
    const PAWN_WHITE_ATTACK_BITBOARDS: &'static [Self::Bitboard] = &HUNDRED_PAWN_WHITE_ATTACK_BITBOARDS;
}
