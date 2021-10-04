// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate contains a binary calling the IPOG implementation provided in [ipog_multi].

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]

use std::path::PathBuf;
use std::sync::Arc;

use libreca::common::{Id, time_it};
use libreca::main;
use libreca::writer::write_result;
use libreca::sut::{ConstrainedSUT, Solver, SolverImpl, SUT};

/// Run the multithreaded IPOG for a SUT without constraints.
fn unconstrained<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(
    mut sut: SUT<ValueId, ParameterId>, output_file: PathBuf,
) -> Result<(), String> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let mca = time_it!(
        ipog_multi::unconstrained::UnconstrainedMCIPOG::<
            ValueId,
            ParameterId,
            STRENGTH,
        >::run(&mut sut),
        "Generation"
    );
    time_it!(
        write_result(&sut, mca, output_file).map_err(|e| e.to_string()),
        "Writing"
    )
}

/// Run the multithreaded IPOG for a SUT with constraints.
fn constrained<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(
    mut sut: ConstrainedSUT<ValueId, ParameterId>, output_file: PathBuf,
) -> Result<(), String> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let solver_init = SolverImpl::default_init();
    let solver = sut.get_solver::<SolverImpl>(&solver_init);
    let sut = Arc::new(sut);
    let mca = time_it!(
        ipog_multi::constrained::ConstrainedMCIPOG::<
            ValueId,
            ParameterId,
            STRENGTH,
        >::run(sut.clone(), solver),
        "Generation"
    );
    time_it!(
        write_result(&sut.sub_sut, mca, output_file).map_err(|e| e.to_string()),
        "Writing"
    )
}

main!(
    /// Run the multithreaded IPOG for the given command line arguments.
    unconstrained, constrained
);
