// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::sync::Arc;
use std::thread::spawn;

use crate::{MiniSatSolver, Solver, parse_constrained, ConstrainedSUT};

#[test]
fn test_threads() {
    let mut sut = parse_constrained("\
    p0: v0, v1;\
    p1: v0, v1, v2;\
    p2: v0, v1, v2;\
    p3: v0, v1;\
    p4: v0, v1;\
    $assert p1=v0 => p2=v1;").expect("Parsing went wrong?");
    sut.get_solver::<MiniSatSolver>(&());

    let sut: Arc<ConstrainedSUT<usize, usize>> = Arc::new(sut);

    let local_sut = sut.clone();
    spawn(move || {
        let mut solver = MiniSatSolver::new(&local_sut, &());
        assert!(solver.check_row(&[0_usize, 0, 0, 0, 0]));
    }).join().unwrap();
}
