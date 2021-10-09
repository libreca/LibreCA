// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

extern crate test;
use test::Bencher;
use common::u_vec;
use super::*;

#[test]
fn test_generate_pc_list_3_5() {
    let result = PCList::<usize, u64, 3>::new(5);
    assert_eq!(result.pcs.len(), 6);
    assert_eq!(calculate_length(3, 4), 6);

    assert_eq!(
        result.pcs,
        u_vec![[0, 1], [0, 2], [1, 2], [0, 3], [1, 3], [2, 3],]
    )
}

#[test]
fn test_generate_pc_list_2_5() {
    let result = PCList::<usize, u64, 2>::new(5);
    assert_eq!(calculate_length(2, 4), 4);
    assert_eq!(result.pcs.len(), 4);

    assert_eq!(result.pcs, u_vec![[0], [1], [2], [3],])
}

#[test]
fn test_generate_pc_list_6_8() {
    let result = PCList::<usize, u64, 6>::new(8);
    assert_eq!(result.pcs.len(), 21);
    assert_eq!(calculate_length(6, 7), 21);

    let check = u_vec![
        [0, 1, 2, 3, 4],
        [0, 1, 2, 3, 5],
        [0, 1, 2, 4, 5],
        [0, 1, 3, 4, 5],
        [0, 2, 3, 4, 5],
        [1, 2, 3, 4, 5],
        [0, 1, 2, 3, 6],
        [0, 1, 2, 4, 6],
        [0, 1, 2, 5, 6],
        [0, 1, 3, 4, 6],
        [0, 1, 3, 5, 6],
        [0, 1, 4, 5, 6],
        [0, 2, 3, 4, 6],
        [0, 2, 3, 5, 6],
        [0, 2, 4, 5, 6],
        [0, 3, 4, 5, 6],
        [1, 2, 3, 4, 6],
        [1, 2, 3, 5, 6],
        [1, 2, 4, 5, 6],
        [1, 3, 4, 5, 6],
        [2, 3, 4, 5, 6],
    ];

    assert_eq!(check, result.pcs);
}

#[test]
fn test_generate_pc_list_3_14() {
    let result = PCList::<usize, u64, 3>::new(14);
    assert_eq!(result.pcs.len(), 78);
    assert_eq!(calculate_length(3, 13), 78);

    let check = u_vec![
        [0, 1],
        [0, 2],
        [1, 2],
        [0, 3],
        [1, 3],
        [2, 3],
        [0, 4],
        [1, 4],
        [2, 4],
        [3, 4],
        [0, 5],
        [1, 5],
        [2, 5],
        [3, 5],
        [4, 5],
        [0, 6],
        [1, 6],
        [2, 6],
        [3, 6],
        [4, 6],
        [5, 6],
        [0, 7],
        [1, 7],
        [2, 7],
        [3, 7],
        [4, 7],
        [5, 7],
        [6, 7],
        [0, 8],
        [1, 8],
        [2, 8],
        [3, 8],
        [4, 8],
        [5, 8],
        [6, 8],
        [7, 8],
        [0, 9],
        [1, 9],
        [2, 9],
        [3, 9],
        [4, 9],
        [5, 9],
        [6, 9],
        [7, 9],
        [8, 9],
        [0, 10],
        [1, 10],
        [2, 10],
        [3, 10],
        [4, 10],
        [5, 10],
        [6, 10],
        [7, 10],
        [8, 10],
        [9, 10],
        [0, 11],
        [1, 11],
        [2, 11],
        [3, 11],
        [4, 11],
        [5, 11],
        [6, 11],
        [7, 11],
        [8, 11],
        [9, 11],
        [10, 11],
        [0, 12],
        [1, 12],
        [2, 12],
        [3, 12],
        [4, 12],
        [5, 12],
        [6, 12],
        [7, 12],
        [8, 12],
        [9, 12],
        [10, 12],
        [11, 12],
    ];

    assert_eq!(check, result.pcs);
}

#[bench]
fn bench_generation_struct(bencher: &mut Bencher) {
    let max_pc_count = calculate_length(6, 37);
    bencher.iter(|| {
        let pc_list = PCList::<usize, u64, 6>::new(38);
        assert_eq!(pc_list.pcs.len(), max_pc_count);
    })
}
