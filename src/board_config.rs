use crate::bitboard::{Bitboard, BitboardIndex, U16384, U512};
use crate::chess_move::{BigMove, MoveTrait, RegularMove};
use std::fmt::Debug;

pub trait BoardConfig: Sized + Clone + Default + Debug + Send + Sync {
    const WIDTH: usize;
    const HEIGHT: usize;
    const AREA: usize = Self::WIDTH * Self::HEIGHT;
    type Square: BitboardIndex;
    type Bitboard: Bitboard<Index = Self::Square> + Send + Sync;
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
}
