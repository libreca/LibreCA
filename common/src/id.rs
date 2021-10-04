// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::iter::Step;
use std::ops::{Add, AddAssign, Div, Rem, RemAssign, Sub, SubAssign};

/// This allows for dynamic switching between different types to use for the values and parameters.
pub trait Id:
'static + Default + Copy + Clone + Send + Sync +
Display + Debug + Hash +
Eq + PartialOrd + Step +
Add<Output=Self> + Sub<Output=Self> + Div<Output=Self> + Rem<Output=Self> +
AddAssign + SubAssign + RemAssign
{
    /// The value used as a `dont_care` value.
    ///
    /// This is generally equal to the maximum value of the implementing type.
    fn dont_care() -> Self;

    /// Convert to [usize].
    fn as_usize(self) -> usize;

    /// Convert from [usize].
    fn from_usize(other: usize) -> Self;
}

macro_rules! as_id {
    ($t:ident, $($ts:ident),+) => { as_id!($t); as_id!($($ts),+); };
    ($t:ident) => {
        impl Id for $t {
            #[inline(always)]
            fn dont_care() -> Self { $t::MAX }
            #[inline(always)]
            fn as_usize(self) -> usize { self as usize }
            #[inline(always)]
            fn from_usize(other: usize) -> Self { other as $t }
        }
    };
}

as_id!(u8, u16, u32, u64, u128, usize);
