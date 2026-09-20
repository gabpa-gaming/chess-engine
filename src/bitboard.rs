use std::fmt::Debug;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not,
    Deref, DerefMut, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

pub trait BitboardIndex: Copy + Clone + Debug + Eq + Ord + Send + Sync + 'static {
    fn from_usize(index: usize) -> Self;
    fn as_usize(self) -> usize;
}

macro_rules! impl_bitboard_index {
    ($($type:ty),* $(,)?) => {$ (
        impl BitboardIndex for $type {
            fn from_usize(index: usize) -> Self { index as Self }
            fn as_usize(self) -> usize { self as usize }
        }
    )*};
}

impl_bitboard_index!(u8, u16, u32, usize);

pub trait Bitboard:
    Copy
    + Clone
    + Default
    + Debug
    + Eq
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
    ($($type:ty),* $(,)?) => {$ (
        impl Bitboard for $type {
            type Index = u8;
            type Storage = Self;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const BITS: usize = <$type>::BITS as usize;
            fn from_u64(value: u64) -> Self { value as Self }
            fn bit(index: Self::Index) -> Self { 1 << index }
            fn has(self, index: Self::Index) -> bool { self & Self::bit(index) != 0 }
            fn any(self) -> bool { self != 0 }
            fn first_set(self) -> Option<Self::Index> {
                (self != 0).then(|| self.trailing_zeros() as Self::Index)
            }
        }
    )*};
}

impl_integer_bitboard!(u64, u128);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultiBitboard<const WORDS: usize>(pub [u64; WORDS]);

impl<const WORDS: usize> Default for MultiBitboard<WORDS> {
    fn default() -> Self { Self([0; WORDS]) }
}

impl<const WORDS: usize> Deref for MultiBitboard<WORDS> {
    type Target = [u64; WORDS];
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<const WORDS: usize> DerefMut for MultiBitboard<WORDS> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

pub type U256 = MultiBitboard<4>;
pub type U512 = MultiBitboard<8>;
pub type U1024 = MultiBitboard<16>;
pub type U16384 = MultiBitboard<256>;

impl<const WORDS: usize> Bitboard for MultiBitboard<WORDS> {
    type Index = u16;
    type Storage = [u64; WORDS];
    const ZERO: Self = Self([0; WORDS]);
    const ONE: Self = Self([1; WORDS]);
    const BITS: usize = WORDS * 64;

    fn from_u64(value: u64) -> Self {
        let mut bits = Self::ZERO;
        if WORDS > 0 { bits.0[0] = value; }
        bits
    }

    fn bit(index: Self::Index) -> Self {
        let index = index.as_usize();
        let mut bits = Self::default();
        bits.0[index / 64] = 1 << (index % 64);
        bits
    }

    fn has(self, index: Self::Index) -> bool {
        let index = index.as_usize();
        self.0[index / 64] & (1 << (index % 64)) != 0
    }

    fn any(self) -> bool { self.0.iter().any(|word| *word != 0) }

    fn first_set(self) -> Option<Self::Index> {
        self.0.iter().position(|word| *word != 0).map(|word| Self::Index::from_usize(word * 64 + self.0[word].trailing_zeros() as usize))
    }
}

macro_rules! impl_multiword_ops {
    ($trait:ident, $method:ident, $operator:tt) => {
        impl<const WORDS: usize> $trait for MultiBitboard<WORDS> {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                let mut result = Self::default();
                for index in 0..WORDS { result.0[index] = self.0[index] $operator rhs.0[index]; }
                result
            }
        }
    };
}

impl_multiword_ops!(BitAnd, bitand, &);
impl_multiword_ops!(BitOr, bitor, |);
impl_multiword_ops!(BitXor, bitxor, ^);

impl<const WORDS: usize> Add for MultiBitboard<WORDS> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let mut result = Self::default();
        let mut carry = false;
        for index in 0..WORDS {
            let (sum, carry_left) = self.0[index].overflowing_add(rhs.0[index]);
            let (sum, carry_right) = sum.overflowing_add(carry as u64);
            result.0[index] = sum;
            carry = carry_left || carry_right;
        }
        result
    }
}

impl<const WORDS: usize> Sub for MultiBitboard<WORDS> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let mut result = Self::default();
        let mut borrow = false;
        for index in 0..WORDS {
            let (difference, borrow_left) = self.0[index].overflowing_sub(rhs.0[index]);
            let (difference, borrow_right) = difference.overflowing_sub(borrow as u64);
            result.0[index] = difference;
            borrow = borrow_left || borrow_right;
        }
        result
    }
}

impl<const WORDS: usize> Not for MultiBitboard<WORDS> {
    type Output = Self;
    fn not(self) -> Self {
        let mut result = Self::default();
        for index in 0..WORDS { result.0[index] = !self.0[index]; }
        result
    }
}

impl<const WORDS: usize> Shl<usize> for MultiBitboard<WORDS> {
    type Output = Self;
    fn shl(self, shift: usize) -> Self {
        if shift >= WORDS * 64 { return Self::default(); }
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut result = Self::default();
        for destination in (word_shift..WORDS).rev() {
            let source = destination - word_shift;
            result.0[destination] |= self.0[source] << bit_shift;
            if bit_shift != 0 && source > 0 { result.0[destination] |= self.0[source - 1] >> (64 - bit_shift); }
        }
        result
    }
}

impl<const WORDS: usize> Shr<usize> for MultiBitboard<WORDS> {
    type Output = Self;
    fn shr(self, shift: usize) -> Self {
        if shift >= WORDS * 64 { return Self::default(); }
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut result = Self::default();
        for destination in 0..WORDS - word_shift {
            let source = destination + word_shift;
            result.0[destination] |= self.0[source] >> bit_shift;
            if bit_shift != 0 && source + 1 < WORDS { result.0[destination] |= self.0[source + 1] << (64 - bit_shift); }
        }
        result
    }
}

macro_rules! impl_multiword_assign_ops {
    ($trait:ident, $method:ident, $operator:tt) => {
        impl<const WORDS: usize> $trait for MultiBitboard<WORDS> {
            fn $method(&mut self, rhs: Self) {
                for index in 0..WORDS { self.0[index] $operator rhs.0[index]; }
            }
        }
    };
}

impl_multiword_assign_ops!(BitAndAssign, bitand_assign, &=);
impl_multiword_assign_ops!(BitOrAssign, bitor_assign, |=);
impl_multiword_assign_ops!(BitXorAssign, bitxor_assign, ^=);

impl<const WORDS: usize> AddAssign for MultiBitboard<WORDS> {
    fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
}

impl<const WORDS: usize> SubAssign for MultiBitboard<WORDS> {
    fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
}

impl<const WORDS: usize> ShlAssign<usize> for MultiBitboard<WORDS> {
    fn shl_assign(&mut self, shift: usize) { *self = *self << shift; }
}

impl<const WORDS: usize> ShrAssign<usize> for MultiBitboard<WORDS> {
    fn shr_assign(&mut self, shift: usize) { *self = *self >> shift; }
}

#[cfg(test)]
mod tests {
    use super::{Bitboard, MultiBitboard, U512};

    #[test]
    fn native_bitboards_use_compact_indices() {
        assert_eq!(<u64 as Bitboard>::ONE, 1);
        assert_eq!(<u64 as Bitboard>::bit(63), 1_u64 << 63);
        assert!(<u64 as Bitboard>::bit(63).has(63));
        assert_eq!(<u128 as Bitboard>::bit(127).first_set(), Some(127));
    }

    #[test]
    fn multiword_bitboards_cross_word_boundaries() {
        let bit = U512::bit(400);
        assert!(bit.has(400));
        assert_eq!(bit.first_set(), Some(400));
        assert!((U512::bit(63) << 1).has(64));
        assert!((U512::bit(64) >> 1).has(63));
        assert!((U512::bit(63) + U512::bit(63)).has(64));
        assert!((U512::bit(64) - U512::bit(63)).has(63));
    }

    #[test]
    fn storage_size_is_configurable() {
        assert!(U512::ONE.has(0));
        assert_eq!(std::mem::size_of::<MultiBitboard<4>>(), 32);
        assert_eq!(std::mem::size_of::<U512>(), 64);
    }
}
