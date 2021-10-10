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
//! This crate provides the following optional feature:
//!   * `filter-map` Mark interactions disallowed by the constraints as covered in the [cm::CoverageMap] before beginning the extensions.

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

pub mod unconstrained;

pub mod constrained;
