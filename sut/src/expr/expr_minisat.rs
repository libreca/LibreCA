// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::collections::HashMap;

use minisat::{Bool, Solver};

use common::{Id, UVec};

use crate::expr::{BinOp, BOp, Eq, False, Not, True};

pub(crate) trait ApplyMiniSat {
    fn apply_minisat(&self, parameter_to_id: &HashMap<String, usize>, value_to_id: &UVec<HashMap<String, usize>>, solver: &mut Solver, parameters: &[Vec<Bool>]) -> Bool;
}

impl ApplyMiniSat for False {
    fn apply_minisat(&self, _parameter_to_id: &HashMap<String, usize>, _value_to_id: &UVec<HashMap<String, usize>>, _solver: &mut Solver, _parameters: &[Vec<Bool>]) -> Bool {
        Bool::from(false)
    }
}

impl ApplyMiniSat for True {
    fn apply_minisat(&self, _parameter_to_id: &HashMap<String, usize>, _value_to_id: &UVec<HashMap<String, usize>>, _solver: &mut Solver, _parameters: &[Vec<Bool>]) -> Bool {
        Bool::from(true)
    }
}

impl ApplyMiniSat for Not {
    fn apply_minisat(&self, parameter_to_id: &HashMap<String, usize>, value_to_id: &UVec<HashMap<String, usize>>, solver: &mut Solver, parameters: &[Vec<Bool>]) -> Bool {
        !self.sub.apply_minisat(parameter_to_id, value_to_id, solver, parameters)
    }
}

impl ApplyMiniSat for BinOp {
    fn apply_minisat(&self, parameter_to_id: &HashMap<String, usize>, value_to_id: &UVec<HashMap<String, usize>>, solver: &mut Solver, parameters: &[Vec<Bool>]) -> Bool {
        let left = self.left.apply_minisat(parameter_to_id, value_to_id, solver, parameters);
        let right = self.right.apply_minisat(parameter_to_id, value_to_id, solver, parameters);
        match self.op {
            BOp::And => solver.and_literal(vec![left, right]),
            BOp::Or => solver.or_literal(vec![left, right]),
            BOp::Implies => solver.implies(left, right),
        }
    }
}

impl ApplyMiniSat for Eq {
    fn apply_minisat(&self, parameter_to_id: &HashMap<String, usize>, value_to_id: &UVec<HashMap<String, usize>>, _solver: &mut Solver, parameters: &[Vec<Bool>]) -> Bool {
        let parameter_id = parameter_to_id.get(&self.parameter).expect("Unknown parameter!");
        let value_id = value_to_id[(*parameter_id).as_usize()].get(&self.value).expect("Unknown value!");
        parameters[(*parameter_id).as_usize()][(*value_id).as_usize()]
    }
}
