// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides common features used throughout the IPOG implementations and the data-types of LibreCA.
//!
//! # Features
//!   * `sub-time` Print the timings for all the [sub_time_it] calls.

#![feature(step_trait)]
#![cfg_attr(test, feature(test))]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

pub use id::Id;
pub use u_vec::UVec;
pub use value_generator::ValueGenerator;

mod id;
mod u_vec;
mod value_generator;


/// The minimal supported strength.
pub const MIN_STRENGTH: usize = 2;

/// The maximal supported strength.
pub const MAX_STRENGTH: usize = 12;

/// The text to print when a value is a don't care value.
pub const DONT_CARE_TEXT: &str = "*";

/// Print the time it took to provide the result of the provided expression.
/// Returns the result of the provided expression.
///
/// # Example
/// ```
/// use common::time_it;
///
/// time_it!(0 + 1, "Addition");
/// ```
#[macro_export]
macro_rules! time_it {
    ($code:expr, $text:expr) => {{
        let now = std::time::Instant::now();
        let result = $code;
        let duration = now.elapsed();
        println!("{} takes: {}.{:06}s", $text, duration.as_secs(), duration.subsec_micros());
        result
    }};
}

/// Act like [time_it] if the `sub-time` feature is set. Otherwise return the provided expression.
///
/// # Example
/// ```
/// use common::sub_time_it;
///
/// sub_time_it!(0 + 1, "Addition");
/// ```
///
/// The `sub-time` feature has been set.
#[cfg(feature = "sub-time")]
#[macro_export]
macro_rules! sub_time_it {
    ($code:expr, $text:expr) => {{
        let now = std::time::Instant::now();
        let result = $code;
        let duration = now.elapsed();
        println!("{} takes: {}.{:06}s", $text, duration.as_secs(), duration.subsec_micros());
        result
    }};
}

/// Act like [time_it] if the `sub-time` feature is set. Otherwise return the provided expression.
///
/// # Example
/// ```
/// use common::sub_time_it;
///
/// sub_time_it!(0 + 1, "Addition");
/// ```
///
/// The `sub-time` feature has not been set.
#[cfg(not(feature = "sub-time"))]
#[macro_export]
macro_rules! sub_time_it {
    ($code:expr, $text:expr) => {{$code}};
}

/// This macro calls the given macro for every strength supported.
///
/// # Example
/// ```
/// use common::repeat_strengths;
///
/// macro_rules! temp {
///     ($strength_name:ident, $strength:expr, $text:expr) => {
///         fn $strength_name() {
///             println!("Strength {}; {}", $strength, $text);
///         }
///     };
/// }
///
/// repeat_strengths!(temp, "extra text");
///
/// strength_6();
/// strength_10();
/// ```
#[macro_export]
macro_rules! repeat_strengths {
    ($name:ident $(, $args:tt)*) => {
        $name!(strength_2, 2 $(, $args)*);
        $name!(strength_3, 3 $(, $args)*);
        $name!(strength_4, 4 $(, $args)*);
        $name!(strength_5, 5 $(, $args)*);
        $name!(strength_6, 6 $(, $args)*);
        $name!(strength_7, 7 $(, $args)*);
        $name!(strength_8, 8 $(, $args)*);
        $name!(strength_9, 9 $(, $args)*);
        $name!(strength_10, 10 $(, $args)*);
        $name!(strength_11, 11 $(, $args)*);
        $name!(strength_12, 12 $(, $args)*);
    };
}

#[cfg(test)]
mod test {
    #[test]
    fn test_time_it() {
        let a = time_it!(0, "hi");
        assert_eq!(0, a);
        let a = sub_time_it!(0, "hi");
        assert_eq!(0, a);
    }
}
