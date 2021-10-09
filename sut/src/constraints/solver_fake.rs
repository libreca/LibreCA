// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::{Display, Formatter};

use common::Number;

use crate::{ConstrainedSUT, Solver};

/// This solver does not solve, but instead confirms validity whatever the input.
///
/// To be used for checking SUTs without constriants.
pub struct FakeSolver;

impl Display for FakeSolver {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("<FakeSolver>")
    }
}

impl<'i> Solver<'i> for FakeSolver {
    type Init = ();
    #[inline(always)]
    fn default_init() -> Self::Init { () }
    #[inline(always)]
    fn new<ValueId: Number, ParameterId: Number>(_sut: &ConstrainedSUT<ValueId, ParameterId>, _args: &Self::Init) -> Self { Self }
    #[inline(always)]
    fn check(&mut self) -> bool { true }
    #[inline(always)]
    fn push(&mut self) {}
    #[inline(always)]
    fn push_and_assert_eq<ValueId: Number, ParameterId: Number>(&mut self, _parameter_id: ParameterId, _value_id: ValueId) {}
    #[inline(always)]
    fn push_and_assert_row<ValueId: Number>(&mut self, _row: &[ValueId]) {}
    #[inline(always)]
    fn push_and_assert_row_masked<ValueId: Number, ParameterId: Number>(&mut self, _row: &[ValueId], _pc: &[ParameterId], _at_parameter: usize) {}
    #[inline(always)]
    fn push_and_assert_interaction<ValueId: Number, ParameterId: Number>(&mut self, _pc: &[ParameterId], _at_parameter: usize, _values: &[ValueId]) {}
    #[inline(always)]
    fn pop(&mut self, _num: u32) {}
    #[inline(always)]
    fn pop_all(&mut self, _num: u32) {}
}
