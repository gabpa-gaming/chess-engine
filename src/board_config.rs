use crate::bitboard::{Bitboard, BitboardIndex, ConstBitboardOps};
use crate::chess_move::{BigMove, MoveTrait, RegularMove};
use std::fmt::Debug;

pub trait BoardConfig: Sized + Clone + Default + Debug + Send + Sync {
    const WIDTH: usize;
    const HEIGHT: usize;
    const AREA: usize = Self::WIDTH * Self::HEIGHT;
    type Square: BitboardIndex;
    type Bitboard: Bitboard<Index = Self::Square>
        + Send
        + Sync;
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

