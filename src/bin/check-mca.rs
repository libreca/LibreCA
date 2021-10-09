// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate contains a binary which can check whether a generated MCA is covering for the provided strength.

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]

use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use common::DONT_CARE_TEXT;
use libreca::cm::{BIT_MASK, BIT_SHIFT};
use libreca::common::{Number, u_vec, UVec, ValueGenerator};
use libreca::main;
use libreca::sut::{ConstrainedSUT, Solver, SolverImpl, SUT};

/// This solver does not solve, but instead confirms validity whatever the input.
///
/// To be used for checking SUTs without constriants.
struct FakeSolver;

impl Display for FakeSolver {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("<FakeSolver>")
    }
}

impl<'i> Solver<'i> for FakeSolver {
    type Init = ();
    fn default_init() -> Self::Init { () }
    fn new<ValueId: Number, ParameterId: Number>(_sut: &ConstrainedSUT<ValueId, ParameterId>, _args: &Self::Init) -> Self { Self }
    #[inline(always)]
    fn check(&mut self) -> bool { true }
    fn push(&mut self) {}
    fn push_and_assert_eq<ValueId: Number, ParameterId: Number>(&mut self, _parameter_id: ParameterId, _value_id: ValueId) {}
    fn push_and_assert_row<ValueId: Number>(&mut self, _row: &[ValueId]) {}
    fn push_and_assert_row_masked<ValueId: Number, ParameterId: Number>(&mut self, _row: &[ValueId], _pc: &[ParameterId], _at_parameter: usize) {}
    fn push_and_assert_interaction<ValueId: Number, ParameterId: Number>(&mut self, _pc: &[ParameterId], _at_parameter: usize, _values: &[ValueId]) {}
    fn pop(&mut self, _num: u32) {}
    fn pop_all(&mut self, _num: u32) {}
}

/// Converts an [std::io::Error] to a [String].
fn ioe<V>(result: std::io::Result<V>) -> Result<V, String> {
    result.map_err(|e| e.to_string())
}

/// Does the actual checking of the MCA.
///
/// Call with the [FakeSolver] to check SUTs without constraints.
fn check_mca<'a, S: Solver<'a>, ValueId: Number, ParameterId: Number, const STRENGTH: usize>(
    mut sut: ConstrainedSUT<ValueId, ParameterId>, output_path: PathBuf, solver_init: &'a S::Init,
) -> Result<(), String> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let mut lines = BufReader::new(ioe(File::open(output_path))?).lines().enumerate().skip_while(|(_, l)| match l {
        Ok(l) => l.starts_with("#"),
        Err(_) => false,
    });
    let parameter_line = ioe(lines.next().ok_or("No parameter names line?")?.1)?;
    for (parameter_id, parameter_name) in parameter_line.split(",").enumerate() {
        let old_parameter_id = sut.parameter_to_id[parameter_name];
        if parameter_id != old_parameter_id {
            sut.parameter_to_id.insert(parameter_name.to_string(), parameter_id);
            sut.parameter_to_id.insert(sut.sub_sut.parameter_names[parameter_id].to_string(), old_parameter_id);
            sut.sub_sut.parameters.swap(parameter_id, old_parameter_id);
            sut.sub_sut.parameter_names.swap(parameter_id, old_parameter_id);
            sut.sub_sut.values.swap(parameter_id, old_parameter_id);
            sut.value_to_id.swap(parameter_id, old_parameter_id);
        }
    }

    let expected = sut.sub_sut.parameter_names.unwrap_ref().join(",");
    if &parameter_line != &expected {
        return Err(format!("Parameters incorrect:\n{}\n{}", parameter_line, expected));
    }

    let at_parameter = sut.sub_sut.parameters.len();
    sut.sub_sut.parameters.push(ValueId::from_usize(1));

    let mut solver: S = sut.get_solver::<S>(solver_init);

    let pc_list = libreca::pc_list::PCList::<ParameterId, u128, STRENGTH>::new(sut.sub_sut.parameters.len());
    let mut cm = libreca::cm::CoverageMap::<ValueId, STRENGTH>::new(sut.sub_sut.parameters.clone(), &pc_list);
    cm.initialise(at_parameter);

    let mut row = u_vec![ValueId::default(); at_parameter + 1];

    for (line_number, line) in lines {
        let line = ioe(line)?;
        for (parameter_id, value) in line.split(",").enumerate() {
            row[parameter_id] = if value != DONT_CARE_TEXT {
                ValueId::from_usize(sut.value_to_id[parameter_id][value])
            } else { ValueId::dont_care() };
        }
        if !solver.check_row(row.as_slice()) { return Err(format!("Invalid row on line {}: {}", line_number, line)); }
        unsafe { cm.set_covered_row_simple(at_parameter, &pc_list, pc_list.pcs.len(), row.as_slice()); }
    }

    if cm.is_covered() { return Ok(()); }

    let base_index = cm.sizes[pc_list.pcs.len()][0];
    let block = &mut cm.map[(base_index >> BIT_SHIFT) as usize];
    let mut bit = 1 << (base_index & BIT_MASK);
    while bit != 0 {
        *block |= bit;
        bit <<= 1;
    }

    let mut at_pc = 0;
    for (block_count, mut block) in cm.map.into_iter().enumerate() {
        if block != !0 {
            let mut base_index = block_count << BIT_SHIFT;
            for _ in 0..block.count_zeros() {
                while block & 1 == 1 {
                    base_index += 1;
                    block >>= 1;
                }

                while (cm.sizes[at_pc][0] as usize) <= base_index { at_pc += 1; }
                at_pc -= 1;

                let mut values = [ValueId::default(); STRENGTH];
                let value_generator = ValueGenerator::new(&sut.sub_sut.parameters, at_parameter, &pc_list.pcs[at_pc]);
                value_generator.skip_array(&mut values, ValueId::from_usize(base_index - cm.sizes[at_pc][0] as usize));

                solver.push_and_assert_interaction(&pc_list.pcs[at_pc], at_parameter, &values);
                if solver.check() {
                    println!("{}", parameter_line);
                    println!("[{:?}, {}] {:?}", pc_list.pcs[at_pc], at_parameter, values);
                    return Err(format!("Interaction not covered!"));
                }
                solver.pop_all(1);

                block |= 1;
            }
        }
    }

    return Ok(());
}

/// This is the method checking MCAs for SUTs without constraints.
fn unconstrained<ValueId: Number, ParameterId: Number, const STRENGTH: usize>(
    sut: SUT<ValueId, ParameterId>, output_path: PathBuf,
) -> Result<(), String> where [(); STRENGTH + 1]:, [(); { STRENGTH + 1 } - 1]:, [(); { STRENGTH + 1 } - 2]: {
    let constrained_sut = ConstrainedSUT::wrap_sut(sut);
    let solver_init = FakeSolver::default_init();
    check_mca::<FakeSolver, ValueId, ParameterId, { STRENGTH + 1 }>(constrained_sut, output_path, &solver_init)
}

/// This is the method checking MCAs for SUTs using constraints.
fn constrained<ValueId: Number, ParameterId: Number, const STRENGTH: usize>(
    constrained_sut: ConstrainedSUT<ValueId, ParameterId>, output_path: PathBuf,
) -> Result<(), String> where [(); STRENGTH + 1]:, [(); { STRENGTH + 1 } - 1]:, [(); { STRENGTH + 1 } - 2]: {
    let solver_init = SolverImpl::default_init();
    check_mca::<SolverImpl, ValueId, ParameterId, { STRENGTH + 1 }>(constrained_sut, output_path, &solver_init)
}

main!(
    /// This binary checks whether a generated MCA is covering for the provided strength.
    ///
    /// The implementation is very inefficient, so only use this for small MCAs or be ready to wait.
    unconstrained, constrained
);
