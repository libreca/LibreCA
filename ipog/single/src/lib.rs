// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides a single-threaded implementation of IPOG.
//! Currently it provides support for generating Mixed-level Covering Arrays (MCA)
//! for both unconstrained Systems Under Test (SUT) and constrained SUTs.
//!
//! # Features
//! This crate provides the following optional features:
//!   * `filter-map` Mark interactions disallowed by the constraints as covered in the [cm::CoverageMap] before beginning the extensions.
//!   * `score-single` Always use the naive scoring algorithm.
//!   * `score-double` Switch between the bitwise scoring algorithm and unchecked algorithm when there are no don't-cares.
//!
//! If neither `score-single` or `score-double` are set then the algorithm uses one of the three algorithms:
//!   * If no don't-cares are present: unchecked algorithm [cm::CoverageMap::get_high_score_masked_unchecked].
//!   * If only a few don't-cares are present: naive algorithm [cm::CoverageMap::get_high_score].
//!   * If more don't-cares are present: unchecked algorithm [cm::CoverageMap::get_high_score_masked].

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

pub mod unconstrained;

pub mod constrained;
