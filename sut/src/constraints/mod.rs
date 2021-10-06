// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use common::Number;

pub(crate) mod solver;

#[cfg(feature = "constraints-minisat")]
pub(crate) mod solver_minisat;

#[cfg(feature = "constraints-z3")]
pub(crate) mod solver_z3;

pub(crate) mod solver_not_implemented;


pub(crate) fn find_problem<'i, Solver: solver::Solver<'i>, ValueId: Number>(solver: &mut Solver, row: &[ValueId], mut start: usize, mut end: usize) -> usize {
    while start <= end {
        let mid = (start + end) / 2;
        if solver.check_row(&row[..mid]) {
            start = mid + 1;
        } else {
            end = mid - 1;
        }
    }

    debug_assert!(start >= row.len() || !solver.check_row(&row[..start]));
    debug_assert!(solver.check_row(&row[..start - 1]));

    start
}

#[cfg(all(test, feature = "constraints-minisat"))]
mod test;
