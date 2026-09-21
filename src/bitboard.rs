use std::fmt::Debug;
use std::ops::{
    Add, AddAssign,
    BitAnd, BitAndAssign,
    BitOr, BitOrAssign,
    BitXor, BitXorAssign,
    Not,
    Shl, ShlAssign,
    Shr, ShrAssign,
    Sub, SubAssign,
};

pub trait BitboardIndex:
    Copy
    + Clone
    + Debug
    + Eq
    + Ord
    + Send
    + Sync
    + 'static
{
    fn from_usize(index: usize) -> Self;
    fn as_usize(self) -> usize;
}

macro_rules! impl_bitboard_index {
    ($($type:ty),* $(,)?) => {
        $(
            impl BitboardIndex for $type {
                fn from_usize(index: usize) -> Self {
                    index as Self
                }

                fn as_usize(self) -> usize {
                    self as usize
                }
            }
        )*
    };
}

impl_bitboard_index!(u8, u16, u32, usize);


mod sealed {
    pub trait Sealed {}

    impl Sealed for u64 {}
    impl Sealed for u128 {}
}

pub const trait ConstBitboardOps:
    sealed::Sealed + Copy
{
    fn const_bit(index: usize) -> Self;
    fn const_or(self, rhs: Self) -> Self;
}

const impl ConstBitboardOps for u64 {
    fn const_bit(index: usize) -> Self {
        if index >= 64 {
            0
        } else {
            1u64 << index
        }
    }

    fn const_or(self, rhs: Self) -> Self {
        self | rhs
    }
}

const impl ConstBitboardOps for u128 {
    fn const_bit(index: usize) -> Self {
        if index >= 128 {
            0
        } else {
            1u128 << index
        }
    }

    fn const_or(self, rhs: Self) -> Self {
        self | rhs
    }
}

pub trait Bitboard:
    sealed::Sealed
    + const ConstBitboardOps
    + Copy
    + Clone
    + Default
    + Debug
    + Eq
    + Send
    + Sync
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Not<Output = Self>
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Shl<usize, Output = Self>
    + ShlAssign<usize>
    + Shr<usize, Output = Self>
    + ShrAssign<usize>
    + 'static
{
    type Index: BitboardIndex;
    type Storage: Clone + Debug + Eq + Send + Sync;

    const ZERO: Self;
    const ONE: Self;
    const BITS: usize;

    fn from_u64(value: u64) -> Self;

    fn bit(index: Self::Index) -> Self;

    fn has(self, index: Self::Index) -> bool;

    fn any(self) -> bool;

    fn first_set(self) -> Option<Self::Index>;
}

macro_rules! impl_integer_bitboard {
    ($($type:ty),* $(,)?) => {
        $(
            impl Bitboard for $type {
                type Index = u8;
                type Storage = Self;

                const ZERO: Self = 0;
                const ONE: Self = 1;
                const BITS: usize = <$type>::BITS as usize;

                fn from_u64(value: u64) -> Self {
                    value as Self
                }

                fn bit(index: Self::Index) -> Self {
                    <Self as ConstBitboardOps>::const_bit(
                        index.as_usize()
                    )
                }

                fn has(self, index: Self::Index) -> bool {
                    (self & Self::bit(index)) != Self::ZERO
                }

                fn any(self) -> bool {
                    self != Self::ZERO
                }

                fn first_set(self) -> Option<Self::Index> {
                    if self == Self::ZERO {
                        None
                    } else {
                        Some(
                            Self::Index::from_usize(
                                self.trailing_zeros() as usize
                            )
                        )
                    }
                }
            }
        )*
    };
}

impl_integer_bitboard!(u64, u128);

#[cfg(test)]
mod tests {
    use super::{Bitboard, ConstBitboardOps};

    #[test]
    fn u64_bitboard() {
        assert_eq!(<u64 as Bitboard>::ZERO, 0);
        assert_eq!(<u64 as Bitboard>::ONE, 1);
        assert_eq!(<u64 as Bitboard>::BITS, 64);

        assert_eq!(
            <u64 as Bitboard>::bit(63),
            1u64 << 63
        );

        assert!(<u64 as Bitboard>::bit(63).has(63));
        assert_eq!(
            <u64 as Bitboard>::bit(63).first_set(),
            Some(63)
        );
    }

    #[test]
    fn u128_bitboard() {
        assert_eq!(<u128 as Bitboard>::ZERO, 0);
        assert_eq!(<u128 as Bitboard>::ONE, 1);
        assert_eq!(<u128 as Bitboard>::BITS, 128);

        assert_eq!(
            <u128 as Bitboard>::bit(127),
            1u128 << 127
        );

        assert!(<u128 as Bitboard>::bit(127).has(127));
        assert_eq!(
            <u128 as Bitboard>::bit(127).first_set(),
            Some(127)
        );
    }

    #[test]
    fn const_operations_work() {
        const A: u128 =
            <u128 as ConstBitboardOps>::const_bit(4);

        const B: u128 =
            <u128 as ConstBitboardOps>::const_bit(7);

        const C: u128 =
            <u128 as ConstBitboardOps>::const_or(A, B);

        assert_eq!(
            C,
            (1u128 << 4) | (1u128 << 7)
        );
    }
}