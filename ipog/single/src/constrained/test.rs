// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use mca::MCA;
use sut::{parse_constrained, Solver, SolverImpl};

#[test]
fn test_coverage_map() {
    let mut sut = match parse_constrained(
        "\
    p0: v0, v1;\
    p1: v0, v1, v2;\
    p2: v0, v1, v2;\
    p3: v0, v1;\
    p4: v0, v1;\
    $assert p1=v0 => p2=v1;",
    ) {
        Ok(res) => res,
        Err(e) => panic!("Parsing went wrong? {:?}", e),
    };

    let solver_init = SolverImpl::default_init();
    let mut solver = sut.get_solver::<SolverImpl>(&solver_init);
    let mca = MCA::<usize>::new_constrained::<usize, SolverImpl, 3>(&sut.sub_sut.parameters, &mut solver);

    assert_eq!(mca.array.len(), 2 * 3 * 3 - 2 * 2);

    assert_eq!(
        mca.array,
        u_vec![
            u_vec![0, 0, 0, 0, 0],   // p1=v0  p2=v1  p0=v0
            u_vec![1, 0, 0, !0, !0], // p1=v1  p2=v1  p0=v0
            u_vec![2, 0, 0, !0, !0], // p1=v2  p2=v1  p0=v0
            // u_vec![0, 1, 0, !0, !0],   // p1=v0  p2=v0  p0=v0
            u_vec![1, 1, 0, !0, !0], // p1=v1  p2=v0  p0=v0
            u_vec![2, 1, 0, !0, !0], // p1=v2  p2=v0  p0=v0
            // u_vec![0, 2, 0, !0, !0],   // p1=v0  p2=v2  p0=v0
            u_vec![1, 2, 0, !0, !0], // p1=v1  p2=v2  p0=v0
            u_vec![2, 2, 0, !0, !0], // p1=v2  p2=v2  p0=v0
            u_vec![0, 0, 1, !0, !0], // p1=v0  p2=v1  p0=v1
            u_vec![1, 0, 1, !0, !0], // p1=v1  p2=v1  p0=v1
            u_vec![2, 0, 1, !0, !0], // p1=v2  p2=v1  p0=v1
            // u_vec![0, 1, 1, !0, !0],   // p1=v0  p2=v0  p0=v1
            u_vec![1, 1, 1, !0, !0], // p1=v1  p2=v0  p0=v1
            u_vec![2, 1, 1, !0, !0], // p1=v2  p2=v0  p0=v1
            // u_vec![0, 2, 1, !0, !0],   // p1=v0  p2=v2  p0=v1
            u_vec![1, 2, 1, !0, !0], // p1=v1  p2=v2  p0=v1
            u_vec![2, 2, 1, !0, !0], // p1=v2  p2=v2  p0=v1
        ],
        "{:?} {:?}",
        sut.sub_sut.parameter_names,
        sut.sub_sut.values
    );
}

#[test]
fn test_big() {
    // $assert p0=v1 => !p4=v0;
    // $assert p1=v1 => !p4=v1;
    // $assert p2=v1 => !p4=v2;
    let mut sut = match parse_constrained(
        "
    p0: v0, v1, v2, v3, v4, v5, v6;
    p1: v0, v1, v2, v3, v4, v5;
    p2: v0, v1, v2, v3, v4;
    p3: v0, v1, v2, v3;
    p4: v0, v1, v2;
    p5: v0, v1;

    $assert p0=v1 => !p4=v1;
    $assert p1=v2 => !p4=v0;
    $assert p2=v0 => !p4=v2;
    $assert p0=v0 => p4=v2;
    $assert p0=v3 => p1=v2;
    $assert p0=v3 => p2=v1;
    $assert p0=v4 => p1=v5;
    $assert p0=v4 => p2=v3;
    $assert p0=v6 => p2=v2;
    $assert p0=v6 => p4=v0;
    $assert p1=v0 => p0=v2;
    $assert p1=v0 => p2=v1;
    $assert p1=v0 => p2=v4;
    $assert p1=v1 => p0=v5;
    $assert p1=v3 => p0=v0;
    $assert p1=v3 => p0=v1;
    $assert p1=v4 => p0=v2;
    $assert p1=v4 => p1=v2;
    $assert p1=v5 => p3=v3;
    $assert p2=v1 => p4=v0;
    $assert p2=v2 => p3=v2;
    $assert p2=v3 => p3=v0;
    $assert p2=v3 => p3=v1;
    $assert p2=v4 => p3=v0;
    $assert p3=v0 => p2=v0;
    $assert p3=v1 => p0=v1;
    $assert p3=v3 => p2=v4;
    $assert p4=v1 => p0=v5;
    $assert p4=v1 => p0=v5;
    $assert p4=v1 => p2=v0;
    $assert p4=v2 => p3=v2;
    ",
    ) {
        Ok(res) => res,
        Err(e) => panic!("Parsing went wrong? {:?}", e),
    };

    let solver_init = SolverImpl::default_init();
    let mut solver = sut.get_solver::<SolverImpl>(&solver_init);
    let mca = MCA::<usize>::new_constrained::<usize, SolverImpl, 4>(&sut.sub_sut.parameters, &mut solver);

    assert_eq!(
        mca.array,
        u_vec![
            u_vec![0, 0, 0, 0, 0, 0],   //  1: p0=v0 p1=v2 p2=v2 p3=v2
            u_vec![1, 0, 0, 0, !0, !0], //  2: p0=v1 p1=v2 p2=v2 p3=v2
            u_vec![2, 0, 0, 0, !0, !0], //  3: p0=v2 p1=v2 p2=v2 p3=v2
            u_vec![5, 0, 0, 0, !0, !0], //  4: p0=v5 p1=v2 p2=v2 p3=v2
            u_vec![5, 1, 0, 0, !0, !0], //  5: p0=v5 p1=v1 p2=v2 p3=v2
            u_vec![5, 1, 1, 0, !0, !0], //  6: p0=v5 p1=v1 p2=v1 p3=v2
            u_vec![5, 0, 2, 0, !0, !0], //  7: p0=v5 p1=v2 p2=v0 p3=v2
            u_vec![5, 1, 2, 0, !0, !0], //  8: p0=v5 p1=v1 p2=v0 p3=v2
            u_vec![5, 0, 2, 2, !0, !0], //  9: p0=v5 p1=v2 p2=v0 p3=v0
            u_vec![5, 1, 2, 2, !0, !0], // 10: p0=v5 p1=v1 p2=v0 p3=v0
        ],
        "{:?} {:?}",
        sut.sub_sut.parameter_names,
        sut.sub_sut.values
    );
}
