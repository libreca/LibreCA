// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use common::Id;
use crate::{ConstrainedSUT};

/// This trait represents any type of solver and allows for switching between backends without too much effort.
pub trait Solver<'i>: std::fmt::Display {
    /// This is the type of any object that needs to be provided to the constructor.
    ///
    /// The [crate::Z3Solver] requires a context, which can not be created inside the constructor because the solver borrows it.
    type Init: Sized + 'i;

    /// Create the objects required to call the constructor.
    fn default_init() -> Self::Init;

    /// Create a new [Solver]. Normally called by [ConstrainedSUT::get_solver]
    fn new<ValueId: Id, ParameterId: Id>(sut: &ConstrainedSUT<ValueId, ParameterId>, args: &'i Self::Init) -> Self;

    /// Check the current stack for validity.
    fn check(&mut self) -> bool;

    /// Check the current stack for validity and pop the given number of elements from the stack.
    fn check_and_pop(&mut self, num: u32) -> bool {
        let result = self.check();
        self.pop(num);
        result
    }

    /// Check the current stack for validity and pop all elements from the stack.
    ///
    /// The provided number should be equal to the current number of elements on the stack.
    fn check_and_pop_all(&mut self, num: u32) -> bool {
        let result = self.check();
        self.pop_all(num);
        result
    }

    /// Check the given row.
    ///
    /// Pushes the row, checks it, and pop it again.
    ///
    /// Requires an empty stack and leaves an empty stack.
    fn check_row<ValueId: Id>(&mut self, row: &[ValueId]) -> bool {
        self.push_and_assert_row(row);
        self.check_and_pop_all(1)
    }

    /// Check the given row with the provided overrides.
    ///
    /// Pushes the row, sets the overrides, checks it, and pops it all.
    ///
    /// Requires an empty stack and leaves an empty stack.
    fn check_row_overrides<ValueId: Id, ParameterId: Id>(&mut self, row: &[ValueId], pc: &[ParameterId], at_parameter: usize, values: &[ValueId]) -> bool {
        self.push_and_assert_interaction(pc, at_parameter, values);
        self.push_and_assert_row_masked(row, pc, at_parameter);
        self.check_and_pop_all(2)
    }

    /// Push the current state to the stack.
    ///
    /// This method should probably not be called directly.
    fn push(&mut self);

    /// Push and then add an equality assertion to the solver.
    fn push_and_assert_eq<ValueId: Id, ParameterId: Id>(&mut self, parameter_id: ParameterId, value_id: ValueId);

    /// Push and then add an row equality assertion to the solver.
    fn push_and_assert_row<ValueId: Id>(&mut self, row: &[ValueId]);

    // TODO pc has known length
    /// Push and then add an row equality assertion, with exception of the parameters in the provided PC, to the solver.
    fn push_and_assert_row_masked<ValueId: Id, ParameterId: Id>(&mut self, row: &[ValueId], pc: &[ParameterId], at_parameter: usize);

    // TODO pc and values have known length
    /// Push and then add an interaction equality assertion to the solver.
    fn push_and_assert_interaction<ValueId: Id, ParameterId: Id>(&mut self, pc: &[ParameterId], at_parameter: usize, values: &[ValueId]);

    /// Pop the given number of elements from the stack.
    fn pop(&mut self, num: u32);

    /// Pop all elements from the stack.
    ///
    /// The provided number should be equal to the current number of elements on the stack.
    fn pop_all(&mut self, num: u32);
}
