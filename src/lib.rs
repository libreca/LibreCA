// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides two implementations of IPOG:
//!   * [ipog_single] A single-threaded implementation of IPOG.
//!   * [ipog_multi] A multithreaded implementation of IPOG.
//!
//! The other crates included provide the data-types used in these two implementations.
//!
//! # Features
//! This crate provides the following optional features:
//!   * `constraints-common` Feature to remove any errors related to unimplemented solver wrappers.
//!   * `constraints-glucose` Implies MiniSat, but compiles the Glucose drop-in replacement instead.
//!   * `constraints-minisat` Add support for the MiniSat solver.
//!   * `constraints-z3` Add support for the Z3 solver.
//!   * `constraints` Implies `constraints-minisat`.
//!   * `no-sort` Do not sort the parameters based on descending level. IPOG runs better on a sorted SUT.
//!   * `sub-time` Print the timings for all the [common::sub_time_it] calls.
//!   * `no-cycle-split` Do not cycle the division of work between the worker threads in the multithreaded implementation of IPOG.
//!   * `filter-map` Mark interactions disallowed by the constraints as covered in the [cm::CoverageMap] before beginning the extensions.
//!   * `score-single` Always use the naive scoring algorithm.
//!   * `score-double` Switch between the bitwise scoring algorithm and unchecked algorithm when there are no don't-cares.
//!
//! If neither `score-single` or `score-double` are set then the algorithm uses one of the three algorithms:
//!   * If no don't-cares are present: unchecked algorithm [cm::CoverageMap::get_high_score_masked_unchecked].
//!   * If only a few don't-cares are present: naive algorithm [cm::CoverageMap::get_high_score].
//!   * If more don't-cares are present: unchecked algorithm [cm::CoverageMap::get_high_score_masked].

#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

pub use cli;
pub use cm;
pub use common;
pub use ipog_multi;
pub use ipog_single;
pub use mca;
pub use pc_list;
pub use sut;
pub use writer;

/// Create a main method which calls the specified methods with the correct arguments, depending on the provided [sut::SUT].
///
/// # Examples
/// Important use cases are provided in the following examples:
///
/// ## Create the entire main method
/// ```
/// #![allow(incomplete_features)]
/// #![feature(adt_const_params)]
/// #![feature(generic_const_exprs)]
///
/// use std::path::PathBuf;
/// use cli::SUTWrapper;
/// use sut::{ConstrainedSUT, SUT};
/// use common::Number;
/// use libreca::main;
///
/// fn unconstrained_method<ValueId: Number, ParameterId: Number,  const STRENGTH: usize>(sut: SUT<ValueId, ParameterId>, _output_path: PathBuf) -> Result<(), String> {
///     println!("Calling unconstrained IPOG, t={}", STRENGTH);
///     Ok(())
/// }
///
/// fn constrained_method<ValueId: Number, ParameterId: Number,  const STRENGTH: usize>(sut: ConstrainedSUT<ValueId, ParameterId>, _output_path: PathBuf) -> Result<(), String> {
///     println!("Calling constrained IPOG, t={}", STRENGTH);
///     Ok(())
/// }
///
/// // Create a main method which parses the cli arguments and calls the correct method accordingly.
/// main!(unconstrained_method, constrained_method);
/// ```
///
/// ## Provide your own SUTWrapper
/// ```
/// #![allow(incomplete_features)]
/// #![feature(adt_const_params)]
/// #![feature(generic_const_exprs)]
///
/// use std::path::PathBuf;
/// use cli::SUTWrapper;
/// use sut::{SUT, ConstrainedSUT, parse_unconstrained};
/// use common::Number;
/// use libreca::main;
///
/// fn unconstrained_method<ValueId: Number, ParameterId: Number,  const STRENGTH: usize>(sut: SUT<ValueId, ParameterId>, _output_path: PathBuf) {
///     println!("Calling unconstrained IPOG, t={}", STRENGTH);
/// }
///
/// fn constrained_method<ValueId: Number, ParameterId: Number,  const STRENGTH: usize>(sut: ConstrainedSUT<ValueId, ParameterId>, _output_path: PathBuf) {
///     println!("Calling constrained IPOG, t={}", STRENGTH);
/// }
///
/// let sut_wrapper = SUTWrapper::Unconstrained(parse_unconstrained("p1: v1, v2, v3;p2: v1, v2;p3: v1, v2;").unwrap());
/// let output_path = PathBuf::from("result.txt");
///
/// // Call the correct method for the given strength and SUTWrapper
/// main!(call(sut_wrapper, output_path, 2, unconstrained_method, constrained_method));
/// ```
///
/// ## Provide your own SUT
/// ```
/// #![allow(incomplete_features)]
/// #![feature(adt_const_params)]
/// #![feature(generic_const_exprs)]
///
/// use std::path::PathBuf;
/// use sut::{SUT, ConstrainedSUT, parse_unconstrained};
/// use common::Number;
/// use libreca::main;
///
/// fn unconstrained_method<ValueId: Number, ParameterId: Number,  const STRENGTH: usize>(sut: SUT<ValueId, ParameterId>, _output_path: PathBuf) {
///     println!("Calling unconstrained IPOG, t={}", STRENGTH);
/// }
///
/// let sut = parse_unconstrained("p1: v1, v2, v3;p2: v1, v2;p3: v1, v2;").unwrap();
/// let output_path = PathBuf::from("result.txt");
///
/// // Call the correct method for the given SUTWrapper and strength
/// main!(call_constraints(sut, output_path, 2, unconstrained_method));
/// ```
#[macro_export]
macro_rules! main {
    (call_constraints($sut:expr, $output_path:expr, $strength_variable:expr, $method:ident)) => {
        common::repeat_strengths!(main, $sut, $output_path, $strength_variable, $method);
        panic!("Support for the given strength and/or SUT is not precompiled in this version.");
    };

    (call_parameters<$v:tt, {$($ps:tt),+}>($strength:expr, $sut:expr, $output_path:expr, $method:ident)) => {
        $(
            if $sut.parameters_fit::<$ps>().is_ok() { return $method::<$v, $ps, $strength>($sut.mutate(), $output_path); }
        )+
    };
    (call_strength<{$v:tt}, {$($ps:tt),+}>($strength:expr, $sut:expr, $output_path:expr, $method:ident)) => {
        if $sut.values_fit::<$v>().is_ok() {
            main!(call_parameters<$v, {$($ps),+}>($strength, $sut, $output_path, $method));
        }
    };
    (call_strength<{$v:tt, $($vs:tt),*}, {$($ps:tt),+}>($strength:expr, $sut:expr, $output_path:expr, $method:ident)) => {
        main!(call_strength<{$v}, {$($ps),+}>($strength, $sut, $output_path, $method));
        main!(call_strength<{$($vs:tt),*}, {$($ps),+}>($strength, $sut, $output_path, $method));
    };

    (call($sut_wrapper:expr, $output_path:expr, $strength_variable:expr, $unconstrained:ident, $constrained:ident)) => {
        match $sut_wrapper {
            cli::SUTWrapper::Unconstrained(sut) => {
                main!(call_constraints(sut, $output_path, $strength_variable, $unconstrained));
            }
            cli::SUTWrapper::Constrained(sut) => {
                main!(call_constraints(sut, $output_path, $strength_variable, $constrained));
            }
        }
    };

    ($strength_name:ident, $strength:expr, $sut:expr, $output_path:expr, $strength_variable:expr, $method:ident) => {
        if $strength == $strength_variable {
            main!(call_strength<{u8}, {u8}>($strength, $sut, $output_path, $method));
        }
    };

    ($(#[$outer:meta])* $unconstrained:ident, $constrained:ident) => {
        $(#[$outer])*
        fn main() -> Result<(), String> {
            let (sut_wrapper, output_path, strength) = common::time_it!(cli::parse_arguments(file!(), cli::crate_version!()), "Parsing")?;
            main!(call(sut_wrapper, output_path, strength, $unconstrained, $constrained));
        }
    };
}
