// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::Display;
use std::fmt::Formatter;

use common::Number;

use crate::{ConstrainedSUT, Solver};

/// This [Solver] is not a solver, and functions merely as a placeholder when no Solvers are compiled.
pub struct NotASolver;

impl<'ctx> Solver<'ctx> for NotASolver {
    type Init = ();

    fn default_init() -> Self::Init {
        unimplemented!("This program was not compiled with support for constraints.")
    }

    fn new<ValueId: Number, ParameterId: Number>(_sut: &ConstrainedSUT<ValueId, ParameterId>, _args: &'ctx Self::Init) -> Self {
        unimplemented!("This program was not compiled with support for constraints.")
    }

    fn check(&mut self) -> bool {
        unimplemented!()
    }

    fn push(&mut self) {
        unimplemented!()
    }

    fn push_and_assert_eq<ValueId: Number, ParameterId: Number>(&mut self, _parameter_id: ParameterId, _value_id: ValueId) {
        unimplemented!()
    }

    fn push_and_assert_row<ValueId: Number>(&mut self, _row: &[ValueId]) {
        unimplemented!()
    }

    fn push_and_assert_row_masked<ValueId: Number, ParameterId: Number>(&mut self, _row: &[ValueId], _pc: &[ParameterId], _at_parameter: usize) {
        unimplemented!()
    }

    fn push_and_assert_interaction<ValueId: Number, ParameterId: Number>(&mut self, _pc: &[ParameterId], _at_parameter: usize, _values: &[ValueId]) {
        unimplemented!()
    }

    fn pop(&mut self, _num: u32) {
        unimplemented!()
    }

    fn pop_all(&mut self, _num: u32) {
        unimplemented!()
    }
}

impl Display for NotASolver {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}
