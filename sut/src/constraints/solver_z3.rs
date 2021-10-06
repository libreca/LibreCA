// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::collections::HashMap;
use std::fmt::{Error, Formatter};
use std::fmt::Display;

use z3::ast::Bool;
use z3::SatResult;

use common::Number;

use crate::{ConstrainedSUT, Solver};
use crate::expr::expr_z3::CHelpers;

type Enumeration<'ctx> = (z3::Sort<'ctx>, Vec<z3::FuncDecl<'ctx>>, Vec<z3::FuncDecl<'ctx>>);

fn add_testers<'ctx, ValueId: Number, ParameterId: Number>(context: &'ctx z3::Context, helpers: &mut CHelpers<'ctx, '_>, parameter_id: ParameterId, parameter_level: ValueId, enumeration: &Enumeration<'ctx>) {
    let datatype: z3::ast::Dynamic<'ctx> = z3::ast::Datatype::new_const(context, format!("p{}", parameter_id), &enumeration.0).into();
    let mut value_testers: Vec<z3::ast::Bool<'ctx>> = Vec::with_capacity(parameter_level.as_usize());
    for value in enumeration.2.iter() {
        value_testers.push(value.apply(&[&datatype]).as_bool().unwrap())
    }
    helpers.value_testers.push(value_testers);
}

/// This solver uses Z3 to provide the required SAT solving features.
///
/// Uses [z3] as a backend.
pub struct Z3Solver<'ctx> {
    solver: z3::Solver<'ctx>,
    parameters: Vec<Vec<Bool<'ctx>>>,
}

fn create_parameters<'ctx, ValueId: Number, ParameterId: Number>(sut: &ConstrainedSUT<ValueId, ParameterId>, context: &'ctx z3::Context, helpers: &mut CHelpers<'ctx, '_>) {
    let mut types: HashMap<ValueId, Enumeration<'ctx>> = HashMap::new();
    for (parameter_id, parameter_level) in sut.sub_sut.parameters.iter().enumerate() {
        match types.get(parameter_level) {
            None => {
                let name = format!("Ex{}", parameter_level);
                let values: Vec<z3::Symbol> = (0..(*parameter_level).as_usize()).map(|v| format!("{}_v{}", name, v).into()).collect();
                let enumeration = z3::Sort::enumeration(&context, name.into(), &values);
                add_testers(context, helpers, ParameterId::from_usize(parameter_id), *parameter_level, &enumeration);

                types.insert(*parameter_level, enumeration);
            }
            Some(enumeration) => {
                add_testers(context, helpers, ParameterId::from_usize(parameter_id), *parameter_level, enumeration);
            }
        }
    }
}

impl<'ctx> Solver<'ctx> for Z3Solver<'ctx> {
    type Init = z3::Context;

    fn default_init() -> Self::Init {
        let mut config = z3::Config::new();
        config.set_model_generation(false);
        config.set_proof_generation(false);
        config.set_debug_ref_count(false);
        z3::Context::new(&config)
    }

    fn new<ValueId: Number, ParameterId: Number>(sut: &ConstrainedSUT<ValueId, ParameterId>, context: &'ctx Self::Init) -> Self {
        let mut helpers = CHelpers {
            context,
            parameter_to_id: &sut.parameter_to_id,
            value_to_id: &sut.value_to_id,
            value_testers: Default::default(),
        };

        create_parameters(sut, context, &mut helpers);

        let solver = z3::Solver::new(context);
        for constraint in sut.constraints.iter() {
            solver.assert(&constraint.apply_z3(&helpers));
        }

        Self {
            solver,
            parameters: helpers.value_testers,
        }
    }

    fn check(&mut self) -> bool {
        let result = self.solver.check();
        debug_assert_ne!(result, SatResult::Unknown);
        result != SatResult::Unsat
    }

    fn push(&mut self) {
        self.solver.push();
    }

    fn push_and_assert_eq<ValueId: Number, ParameterId: Number>(&mut self, parameter_id: ParameterId, value_id: ValueId) {
        self.push();
        self.save_assert_eq(parameter_id.as_usize(), value_id.as_usize());
    }

    fn push_and_assert_row<ValueId: Number>(&mut self, row: &[ValueId]) {
        self.push();
        for (value, testers) in row.iter().zip(self.parameters.iter()) {
            if let Some(tester) = testers.get((*value).as_usize()) {
                self.solver.assert(tester);
            }
        }
    }

    fn push_and_assert_row_masked<ValueId: Number, ParameterId: Number>(&mut self, row: &[ValueId], pc: &[ParameterId], at_parameter: usize) {
        self.push();
        let mut pc_values = pc.iter().peekable();
        for (parameter_id, (value, testers)) in row.iter().take(at_parameter).zip(self.parameters.iter()).enumerate() {
            if let Some(&&parameter) = pc_values.peek() {
                if parameter == ParameterId::from_usize(parameter_id) {
                    pc_values.next();
                    continue;
                }
            }
            if let Some(tester) = testers.get((*value).as_usize()) {
                self.solver.assert(tester);
            }
        }
    }

    fn push_and_assert_interaction<ValueId: Number, ParameterId: Number>(&mut self, pc: &[ParameterId], at_parameter: usize, values: &[ValueId]) {
        debug_assert_eq!(pc.len() + 1, values.len());

        self.push();
        for (&parameter, &value) in pc.iter().zip(values.iter()) {
            self.force_assert_eq(parameter.as_usize(), value.as_usize());
        }
        self.force_assert_eq(at_parameter, values[values.len() - 1].as_usize());
    }

    fn pop(&mut self, num: u32) {
        self.solver.pop(num);
    }

    fn pop_all(&mut self, num: u32) {
        self.solver.pop(num);
    }
}

impl Z3Solver<'_> {
    fn save_assert_eq(&mut self, parameter_id: usize, value_id: usize) {
        if let Some(tester) = self.parameters[parameter_id].get(value_id) {
            self.solver.assert(&tester);
        }
    }

    fn force_assert_eq(&mut self, parameter_id: usize, value_id: usize) {
        self.solver.assert(&self.parameters[parameter_id][value_id]);
    }
}

impl Display for Z3Solver<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.solver.fmt(f)
    }
}
