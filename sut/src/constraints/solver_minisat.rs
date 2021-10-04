// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::{Debug, Display};

use minisat::Bool;
use std::fmt::{Error, Formatter};
use common::Id;

use crate::{ConstrainedSUT, Solver};

/// This solver uses MiniSat or Glucose to provide the required SAT solving features.
///
/// Uses [minisat] as the backend.
pub struct MiniSatSolver {
    solver: minisat::Solver,
    parameters: Vec<Vec<Bool>>,
    values: Vec<Bool>,
    way_points: Vec<usize>,
}

impl<'i> Solver<'i> for MiniSatSolver {
    type Init = ();

    fn default_init() -> Self::Init {}

    fn new<ValueId: Id, ParameterId: Id>(sut: &ConstrainedSUT<ValueId, ParameterId>, _args: &'i Self::Init) -> Self {
        let mut solver = minisat::Solver::new();
        let mut parameters = Vec::with_capacity(sut.sub_sut.parameters.len());

        for &parameter_level in sut.sub_sut.parameters.iter() {
            let m_parameter = minisat::symbolic::Symbolic::new(&mut solver, (0..parameter_level.as_usize()).collect());
            let mut value_testers = Vec::with_capacity(parameter_level.as_usize());
            for value in 0..parameter_level.as_usize() {
                value_testers.push(m_parameter.has_value(&value));
            }
            parameters.push(value_testers);
        }

        if !sut.constraints.is_empty() {
            for constraint in sut.constraints.iter() {
                let clause = vec![constraint.apply_minisat(&sut.parameter_to_id, &sut.value_to_id, &mut solver, &parameters)];
                solver.add_clause(clause);
            }
        }

        Self { solver, parameters, values: Vec::with_capacity(sut.sub_sut.parameters.len()), way_points: Vec::with_capacity(sut.sub_sut.parameters.len()) }
    }

    fn check(&mut self) -> bool {
        if !self.values.is_empty() {
            self.solver.solve_under_borrowed_assumptions(&self.values).is_ok()
        } else {
            self.solver.solve().is_ok()
        }
    }

    fn push(&mut self) {
        self.way_points.push(self.values.len());
    }

    fn push_and_assert_eq<ValueId: Id, ParameterId: Id>(&mut self, parameter_id: ParameterId, value_id: ValueId) {
        self.push();
        if let Some(&tester) = self.parameters[parameter_id.as_usize()].get(value_id.as_usize()) {
            self.values.push(tester);
        }
    }

    fn push_and_assert_row<ValueId: Id>(&mut self, row: &[ValueId]) {
        debug_assert!(self.values.is_empty());
        debug_assert!(self.way_points.is_empty());
        self.push();

        for (value, testers) in row.iter().zip(self.parameters.iter()) {
            if let Some(tester) = testers.get((*value).as_usize()) {
                self.values.push(*tester);
            }
        }
    }

    fn push_and_assert_row_masked<ValueId: Id, ParameterId: Id>(&mut self, row: &[ValueId], pc: &[ParameterId], at_parameter: usize) {
        self.push();
        let mut pc_values = pc.iter().peekable();
        for (parameter_id, (value, testers)) in row.iter().take(at_parameter).zip(self.parameters.iter()).enumerate() {
            if let Some(&&parameter) = pc_values.peek() {
                if parameter == ParameterId::from_usize(parameter_id) {
                    pc_values.next();
                    continue;
                }
            }
            if let Some(&tester) = testers.get((*value).as_usize()) {
                self.values.push(tester);
            }
        }
    }

    fn push_and_assert_interaction<ValueId: Id, ParameterId: Id>(&mut self, pc: &[ParameterId], at_parameter: usize, values: &[ValueId]) {
        debug_assert_eq!(pc.len() + 1, values.len());
        debug_assert_eq!(self.values.len(), 0);
        self.push();

        for (&parameter, &value) in pc.iter().zip(values.iter()) {
            self.values.push(self.parameters[parameter.as_usize()][value.as_usize()]);
        }
        self.values.push(self.parameters[at_parameter][values[values.len() - 1].as_usize()]);
    }

    fn pop(&mut self, num: u32) {
        debug_assert_ne!(num, 0);
        debug_assert!(self.way_points.len() >= num as usize);
        self.way_points.truncate(self.way_points.len() - num as usize + 1);
        let new_len = self.way_points.pop().unwrap_or(0);
        self.values.truncate(new_len);
    }

    fn pop_all(&mut self, num: u32) {
        debug_assert_eq!(self.way_points.len(), num as usize);
        self.values.clear();
        self.way_points.clear();
    }
}

impl Display for MiniSatSolver {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Debug::fmt(&self.solver, f)
    }
}
