#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveFlag {
    Quiet = 0,
    Capture = 1,
    DoublePawnPush = 2,
    EnPassant = 3,
    Castling = 4,
    Promotion = 5,
}

pub trait MoveTrait<C: BoardConfig + Clone + Debug>
where
    [(); C::AREA]: Sized,
{
    fn new(from: C::Square, to: C::Square, flag: MoveFlag, promotion: PieceType<C>) -> Self;
    fn from(&self) -> C::Square;
    fn to(&self) -> C::Square;
    fn flag(&self) -> MoveFlag;
    fn promotion(&self) -> PieceType<C>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BigMove<C: BoardConfig = BigVariant>
where
    [(); C::AREA]: Sized,
{
    pub from: C::Square,
    pub to: C::Square,
    pub flag: MoveFlag,
    pub promotion: PieceType<C>,
}

impl<C: BoardConfig> MoveTrait<C> for BigMove<C>
where
    [(); C::AREA]: Sized,
{
    fn new(from: C::Square, to: C::Square, flag: MoveFlag, promotion: PieceType<C>) -> Self {
        Self {
            from,
            to,
            flag,
            promotion,
        }
    }

    fn from(&self) -> C::Square {
        self.from
    }

    fn to(&self) -> C::Square {
        self.to
    }

    fn flag(&self) -> MoveFlag {
        self.flag
    }

    fn promotion(&self) -> PieceType<C> {
        self.promotion.clone()
    }
}

use crate::bishop::Bishop;
use crate::board_config::{BigVariant, BoardConfig, RegularVariant};
use crate::knight::Knight;
use crate::piece::PieceType;
use crate::queen::Queen;
use crate::rook::Rook;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegularMove(pub u16);

impl MoveTrait<RegularVariant> for RegularMove {
    fn new(from: u8, to: u8, flag: MoveFlag, promotion: PieceType<RegularVariant>) -> Self {
        let encoded_flag_promo = match flag {
            MoveFlag::Promotion => match promotion {
                PieceType::Queen(_) => 5,
                PieceType::Rook(_) => 6,
                PieceType::Bishop(_) => 7,
                PieceType::Knight(_) => 8,
                _ => 0,
            },
            MoveFlag::Quiet => 0,
            MoveFlag::Capture => 1,
            MoveFlag::DoublePawnPush => 2,
            MoveFlag::EnPassant => 3,
            MoveFlag::Castling => 4,
            _ => 0,
        };

        let packed = (from as u16) | ((to as u16) << 6) | ((encoded_flag_promo as u16) << 12);

        Self(packed)
    }

    fn from(&self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    fn to(&self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }

    fn flag(&self) -> MoveFlag {
        let upper = (self.0 >> 12) & 0x0F;
        match upper {
            0 => MoveFlag::Quiet,
            1 => MoveFlag::Capture,
            2 => MoveFlag::DoublePawnPush,
            3 => MoveFlag::EnPassant,
            4 => MoveFlag::Castling,
            5..=8 => MoveFlag::Promotion,
            _ => MoveFlag::Quiet,
        }
    }

    fn promotion(&self) -> PieceType<RegularVariant> {
        let upper = (self.0 >> 12) & 0x0F;
        match upper {
            5 => PieceType::Queen(Queen::default()),
            6 => PieceType::Rook(Rook::default()),
            7 => PieceType::Bishop(Bishop::default()),
            8 => PieceType::Knight(Knight::default()),
            _ => PieceType::None,
        }
    }
}
