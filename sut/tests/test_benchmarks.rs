// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use test_utils::Walker;

use sut::{ConstrainedSUT, MiniSatSolver, parse_constrained, Solver, Z3Solver};
use common::Number;

#[test]
fn test_benchmarks() {
    let current_dir = std::env::current_dir().unwrap();
    assert!(current_dir.exists(), "{:?}", current_dir);
    let mut count = 0;

    for contents in Walker::new(current_dir.parent().unwrap().into()) {
        match parse_constrained(&contents) {
            Ok(result) => {
                count += 1;
                check_benchmark(result)
            },
            Err(e) => { panic!("{}", e) }
        }
    }

    assert!(count > 20, "Did not find enough benchmarks? Successfully parsed {} benchmarks.", count);
}

fn check_benchmark(result: ConstrainedSUT<usize, usize>) {
    if result.sub_sut.parameters_fit::<u8>().is_ok() {
        check_benchmark_p::<u8>(result);
    } else if result.sub_sut.parameters_fit::<u16>().is_ok() {
        check_benchmark_p::<u16>(result);
    } else if result.sub_sut.parameters_fit::<u32>().is_ok() {
        check_benchmark_p::<u32>(result);
    } else if result.sub_sut.parameters_fit::<u64>().is_ok() {
        check_benchmark_p::<u64>(result);
    } else if result.sub_sut.parameters_fit::<u128>().is_ok() {
        check_benchmark_p::<u128>(result);
    } else {
        check_benchmark_p::<usize>(result);
    }
}

fn check_benchmark_p<ParameterId: Number>(result: ConstrainedSUT<usize, usize>) {
    if result.sub_sut.values_fit::<u8>().is_ok() {
        check_benchmark_pv::<u8, ParameterId>(result.mutate());
    } else if result.sub_sut.values_fit::<u16>().is_ok() {
        check_benchmark_pv::<u16, ParameterId>(result.mutate());
    } else if result.sub_sut.values_fit::<u32>().is_ok() {
        check_benchmark_pv::<u32, ParameterId>(result.mutate());
    } else if result.sub_sut.values_fit::<u64>().is_ok() {
        check_benchmark_pv::<u64, ParameterId>(result.mutate());
    } else if result.sub_sut.values_fit::<u128>().is_ok() {
        check_benchmark_pv::<u128, ParameterId>(result.mutate());
    } else {
        check_benchmark_pv(result);
    }
}

fn check_benchmark_pv<ValueId: Number, ParameterId: Number>(mut result: ConstrainedSUT<ValueId, ParameterId>) {
    let row = vec![ValueId::default(); 100];

    assert_ne!(result.sub_sut.parameters.len(), 0);
    for parameter in result.sub_sut.parameters.iter() {
        assert_ne!(*parameter, ValueId::default());
    }

    assert!(result.sub_sut.parameters.len() < row.len());

    let context = Z3Solver::default_init();
    let mut solver: Z3Solver = result.get_solver(&context);
    assert!(solver.check(), "Unsat?\n{}", solver.to_string());
    assert!(solver.check_row(&row[..result.sub_sut.parameters.len()]));

    let mut solver: MiniSatSolver = result.get_solver(&());
    assert!(solver.check(), "Unsat?\n{}", solver.to_string());
    assert!(solver.check_row(&row[..result.sub_sut.parameters.len()]));
}
