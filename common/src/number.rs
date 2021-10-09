// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::{Binary, Debug, Display};
use std::hash::Hash;
use std::iter::Step;
use std::ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Div, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign};

/// This allows for dynamic switching between different types to use for the values and parameters.
pub trait Number:
'static + Default + Copy + Clone + Send + Sync + Sized +
Display + Debug + Binary + Hash +
Eq + PartialOrd + Step +
Add<Output=Self> + Sub<Output=Self> + Div<Output=Self> + Rem<Output=Self> +
BitAnd<Output=Self> + BitOr<Output=Self> + Shl<Output=Self> + Shr<Output=Self> + Not<Output=Self> +
AddAssign + SubAssign + RemAssign + ShlAssign + ShrAssign + BitAndAssign + BitOrAssign
{
    /// The value used as a `dont_care` value.
    ///
    /// This is generally equal to the maximum value of the implementing type.
    fn dont_care() -> Self;

    /// Convert to [usize].
    fn as_usize(self) -> usize;

    /// Convert from [usize].
    fn from_usize(other: usize) -> Self;

    /// Convert from [Number].
    fn from<N: Number>(number: N) -> Self;

    /// See [usize::count_ones].
    fn count_ones(self) -> u32;

    /// Create a mask to get higher and equal bits than the number provided.
    ///
    /// # Example
    /// ```
    /// # use common::Number;
    /// let mask = <u8 as Number>::mask_high(4);
    /// assert_eq!(mask, 0b11110000);
    /// ```
    fn mask_high(shl: usize) -> Self;

    /// Create a mask to get lower bits than the number provided.
    ///
    /// # Example
    /// ```
    /// # use common::Number;
    /// let mask = <u8 as Number>::mask_low(4);
    /// assert_eq!(mask, 0b00001111);
    /// ```
    fn mask_low(shl: usize) -> Self;

    /// Get a number with the given bit set.
    ///
    /// # Example
    /// ```
    /// # use common::Number;
    /// let mask = <u8 as Number>::bit(4);
    /// assert_eq!(mask, 0b00010000);
    /// ```
    fn bit(shl: usize) -> Self;

    /// Return [true] if any of the bits is set.
    fn any(self) -> bool;

    /// Return [true] if none of the bits is set.
    fn none(self) -> bool;

    /// Return [true] if the bit at the given index is set.
    fn get(self, shl: usize) -> bool;
}

macro_rules! as_number {
    ($t:ident, $($ts:ident),+) => { as_number!($t); as_number!($($ts),+); };
    ($t:ident) => {
        impl Number for $t {
            #[inline(always)]
            fn dont_care() -> Self { $t::MAX }
            #[inline(always)]
            fn as_usize(self) -> usize { self as usize }
            #[inline(always)]
            fn from_usize(other: usize) -> Self { other as $t }
            #[inline(always)]
            fn from<N: Number>(number: N) -> Self { Self::from_usize(number.as_usize()) }
            #[inline(always)]
            fn count_ones(self) -> u32 { self.count_ones() }
            #[inline(always)]
            fn mask_high(shl: usize) -> Self  { (!0) << (shl as $t) }
            #[inline(always)]
            fn mask_low(shl: usize) -> Self  { !Self::mask_high(shl) }
            #[inline(always)]
            fn bit(shl: usize) -> Self { 1 << (shl as $t) }
            #[inline(always)]
            fn any(self) -> bool { self != 0 }
            #[inline(always)]
            fn none(self) -> bool { self == 0 }
            #[inline(always)]
            fn get(self, shl: usize) -> bool { ((self >> (shl as $t)) & 1).any() }
        }
    };
}

as_number!(u8, u16, u32, u64, u128, usize);
