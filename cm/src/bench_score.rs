// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

extern crate test;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use test::Bencher;

use lazy_static::lazy_static;

use common::{u_vec, UVec};
use pc_list::PCList;

use crate::CoverageMap;

type BitArray = u64;

const STRENGTH_CONSTANT: usize = 6;

const PARAMETERS_SHORT_LOW: usize = 0;
const PARAMETERS_SHORT_HIGH: usize = 1;
const PARAMETERS_MID_LOW: usize = 2;
const PARAMETERS_MID_HIGH: usize = 3;
const PARAMETERS_LONG_LOW: usize = 4;
const PARAMETERS_LONG_HIGH: usize = 5;

const ROW_A_FILLED: usize = 0;
const ROW_B_ONE_START: usize = 1;
const ROW_C_ONE_MID: usize = 2;
const ROW_D_ONE_END: usize = 3;

// not short
const ROW_D_FOUR: usize = 4;

// only long
const ROW_E_SIX: usize = 5;
const ROW_F_LESS_EMPTY: usize = 6;
const ROW_G_EMPTY: usize = 7;

fn row_to_locations(row: &[usize]) -> BitArray {
    let mut result = !0;
    let mut location = 1;
    for &value in row {
        if value != !0 {
            result -= location;
        }
        location <<= 1;
    }
    result
}

pub trait ScoreGetter {
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        dont_care_locations: BitArray,
        no_dont_cares: BitArray,
    );
}

pub struct Naive;

impl ScoreGetter for Naive {
    #[inline]
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        _dont_care_locations: BitArray,
        _no_dont_cares: BitArray,
    ) {
        unsafe {
            cm.get_high_score(&pc_list, pc_list_len, row, scores);
        }
    }
}

pub struct Checked;

impl ScoreGetter for Checked {
    #[inline]
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        dont_care_locations: BitArray,
        _no_dont_cares: BitArray,
    ) {
        unsafe {
            cm.get_high_score_masked_checked(&pc_list, pc_list_len, row, dont_care_locations, scores);
        }
    }
}

pub struct Unchecked;

impl ScoreGetter for Unchecked {
    #[inline]
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        _dont_care_locations: BitArray,
        _no_dont_cares: BitArray,
    ) {
        unsafe {
            cm.get_high_score_masked_unchecked(&pc_list, pc_list_len, row, scores);
        }
    }
}

pub struct SwitchDouble;

impl ScoreGetter for SwitchDouble {
    #[inline]
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        dont_care_locations: BitArray,
        no_dont_cares: BitArray,
    ) {
        cm.get_high_score_masked(
            &pc_list,
            pc_list_len,
            row,
            dont_care_locations,
            no_dont_cares,
            scores,
        );
    }
}

pub struct SwitchTriple;

impl ScoreGetter for SwitchTriple {
    #[inline]
    fn get_high_score(
        cm: &CoverageMap<usize, 6>,
        pc_list: &PCList<usize, u64, 6>,
        pc_list_len: usize,
        row: &[usize],
        scores: &mut UVec<UVec<BitArray>>,
        dont_care_locations: BitArray,
        no_dont_cares: BitArray,
    ) {
        cm.get_high_score_masked_triple(
            &pc_list,
            pc_list_len,
            row,
            dont_care_locations,
            no_dont_cares,
            scores,
        );
    }
}

lazy_static! {
    static ref INIT: Arc<RwLock<HashMap<usize, Init>>> = Arc::new(RwLock::new(HashMap::new()));
}

struct Init {
    pc_list: PCList<usize, u64, 6>,
    pc_list_len: usize,
    cm: CoverageMap<usize, 6>,
    scores: UVec<UVec<BitArray>>,
    rows: HashMap<usize, (UVec<usize>, BitArray)>,
    no_dont_cares: BitArray,
}

impl Init {
    fn new(parameters: UVec<usize>, rows: HashMap<usize, (UVec<usize>, BitArray)>) -> Self {
        assert_eq!(parameters.len() % 5, 0);
        assert!(rows.values().all(|r| r.0.len() == parameters.len()));
        let at_parameter = parameters.len() - 1;
        let pc_list = PCList::<usize, u64, 6>::new(parameters.len());
        let pc_list_len = pc_list.sizes[at_parameter - STRENGTH_CONSTANT];
        let mut cm = CoverageMap::<usize, 6>::new(parameters.clone(), &pc_list);
        cm.initialise(at_parameter);
        let mut new_value = 0;
        for array in cm.map.iter_mut() {
            *array = new_value;
            new_value = !new_value;
        }
        let scores = u_vec![UVec::with_capacity(pc_list_len); 2];
        let no_dont_cares = !((!0) << at_parameter as BitArray);

        Self {
            pc_list: pc_list,
            pc_list_len: pc_list_len,
            cm,
            scores,
            rows,
            no_dont_cares,
        }
    }
}

fn rows_to_hashmap(rows: UVec<(usize, UVec<usize>)>) -> HashMap<usize, (UVec<usize>, BitArray)> {
    let mut result = HashMap::new();
    for row in rows {
        let locations = row_to_locations(row.1.as_slice());
        assert!(result.insert(row.0, (row.1, locations)).is_none());
    }
    result
}

fn init() {
    let mut init_guard: RwLockWriteGuard<HashMap<usize, Init>> = INIT.write().unwrap();
    if init_guard.is_empty() {
        let short_rows = rows_to_hashmap(u_vec![
            (ROW_A_FILLED, u_vec![5, 2, 1, 0, 1, 1, 0, 0, 1, !0]),
            (ROW_B_ONE_START, u_vec![!0, 1, 1, 0, 1, 0, 0, 0, 1, !0]),
            (ROW_C_ONE_MID, u_vec![5, 2, 1, 1, !0, 0, 0, 0, 1, !0]),
            (ROW_D_ONE_END, u_vec![5, 2, 1, 0, 1, 1, 0, 1, !0, !0]),
            (ROW_F_LESS_EMPTY, u_vec![5, 2, !0, 0, 1, 1, !0, 0, 1, !0]),
            (ROW_G_EMPTY, u_vec![!0, 1, 1, 0, !0, 1, 0, !0, 1, !0]),
        ]);

        assert!(init_guard
            .insert(
                PARAMETERS_SHORT_LOW,
                Init::new(u_vec![7, 3, 2, 2, 2, 2, 2, 2, 2, 2], short_rows.clone()),
            )
            .is_none());
        assert!(init_guard
            .insert(
                PARAMETERS_SHORT_HIGH,
                Init::new(u_vec![15, 10, 6, 6, 5, 4, 3, 3, 2, 2], short_rows),
            )
            .is_none());

        let mid_rows = rows_to_hashmap(u_vec![
            (
                ROW_A_FILLED,
                u_vec![5, 2, 1, 0, 1, 2, 0, 0, 1, 0, 1, 0, 0, 0, !0],
            ),
            (
                ROW_B_ONE_START,
                u_vec![!0, 2, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0, !0],
            ),
            (
                ROW_C_ONE_MID,
                u_vec![5, 2, 1, 2, 0, 1, !0, 1, 1, 0, 1, 0, 0, 0, !0],
            ),
            (
                ROW_D_ONE_END,
                u_vec![5, 2, 1, 0, 1, 2, 0, 1, 1, 0, 1, 0, 0, !0, !0],
            ),
            (
                ROW_D_FOUR,
                u_vec![5, 2, 1, !0, 1, 0, !0, 0, 1, !0, !0, 0, 0, 0, !0],
            ),
            (
                ROW_F_LESS_EMPTY,
                u_vec![2, 1, !0, 1, 1, !0, 1, !0, 1, 1, !0, !0, 0, !0, !0],
            ),
            (
                ROW_G_EMPTY,
                u_vec![!0, 1, !0, 1, !0, !0, !0, 1, 1, !0, 1, !0, 0, !0, !0],
            ),
        ]);

        assert!(init_guard
            .insert(
                PARAMETERS_MID_LOW,
                Init::new(
                    u_vec![7, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2],
                    mid_rows.clone(),
                ),
            )
            .is_none());
        assert!(init_guard
            .insert(
                PARAMETERS_MID_HIGH,
                Init::new(u_vec![15, 6, 6, 5, 4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2], mid_rows),
            )
            .is_none());

        let long_rows = rows_to_hashmap(u_vec![
            (
                ROW_A_FILLED,
                u_vec![5, 2, 1, 0, 1, 2, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, !0],
            ),
            (
                ROW_B_ONE_START,
                u_vec![!0, 2, 1, 0, 1, 2, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, !0],
            ),
            (
                ROW_C_ONE_MID,
                u_vec![5, 2, 1, 0, 1, 2, 0, 0, !0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, !0],
            ),
            (
                ROW_D_ONE_END,
                u_vec![5, 2, 1, 0, 1, 2, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, !0, !0],
            ),
            (
                ROW_D_FOUR,
                u_vec![
                    5, !0, 1, 0, 1, 2, !0, 0, 1, 0, !0, 0, 0, 0, 1, 0, !0, 0, 0, !0,
                ],
            ),
            (
                ROW_E_SIX,
                u_vec![
                    5, !0, 1, !0, 1, !0, 0, !0, 1, 0, 1, 0, 0, !0, !0, 0, 1, 0, 0, !0,
                ],
            ),
            (
                ROW_F_LESS_EMPTY,
                u_vec![
                    !0, 2, !0, 0, !0, !0, 0, 0, !0, 0, !0, 0, 0, !0, !0, 0, !0, !0, 0, !0,
                ],
            ),
            (
                ROW_G_EMPTY,
                u_vec![
                    !0, 2, !0, 0, 1, !0, !0, !0, 1, !0, !0, 0, 0, !0, !0, !0, !0, !0, !0, !0,
                ],
            ),
        ]);

        assert!(init_guard
            .insert(
                PARAMETERS_LONG_LOW,
                Init::new(
                    u_vec![7, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
                    long_rows.clone(),
                ),
            )
            .is_none());
        assert!(init_guard
            .insert(
                PARAMETERS_LONG_HIGH,
                Init::new(
                    u_vec![15, 6, 6, 5, 4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
                    long_rows,
                ),
            )
            .is_none());
    }
}

fn bench_get_score<SG: ScoreGetter>(bencher: &mut Bencher, parameters: usize, row: usize) {
    init();
    let init_guard: RwLockReadGuard<HashMap<usize, Init>> = INIT.read().unwrap();
    let init = init_guard.get(&parameters).unwrap();
    let mut scores = init.scores.clone();
    let row = init.rows.get(&row).unwrap();

    bencher.iter(|| {
        SG::get_high_score(
            &init.cm,
            &init.pc_list,
            init.pc_list_len,
            row.0.as_slice(),
            &mut scores,
            row.1,
            init.no_dont_cares,
        );

        assert!(0 < scores[0].len());
    });
}

fn bench_total<SG: ScoreGetter>(bencher: &mut Bencher, parameters: usize) {
    init();
    let init_guard: RwLockReadGuard<HashMap<usize, Init>> = INIT.read().unwrap();
    let init = init_guard.get(&parameters).unwrap();
    let mut scores = init.scores.clone();

    bencher.iter(|| {
        for row in init.rows.values() {
            SG::get_high_score(
                &init.cm,
                &init.pc_list,
                init.pc_list_len,
                row.0.as_slice(),
                &mut scores,
                row.1,
                init.no_dont_cares,
            );

            assert!(0 < scores[0].len());
        }
    });
}

macro_rules! benches {
    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], [ $p_name:ident = $p_const:ident, $( $n_p_name:ident = $n_p_const:ident ),+], [ $( $n_row_name:ident = $n_row_const:ident ),+ ]) => {
        benches!([$( $n_method_name = $n_method_struct ),*], [ $p_name = $p_const ],            [ $( $n_row_name = $n_row_const ),+]);
        benches!([$( $n_method_name = $n_method_struct ),*], [ $( $n_p_name = $n_p_const ),+ ], [ $( $n_row_name = $n_row_const ),+]);
    };
    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], [ $p_name:ident = $p_const:ident ], [ $( $n_row_name:ident = $n_row_const:ident ),+ ]) => {
        mod $p_name {
            use super::*;
            benches!([$( $n_method_name = $n_method_struct ),*], $p_name = $p_const, [$( $n_row_name = $n_row_const),*]);
        }
    };

    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], [ $p_name:ident = $p_const:ident, $( $n_p_name:ident = $n_p_const:ident ),+]) => {
        benches!([$( $n_method_name = $n_method_struct ),*], [ $p_name = $p_const ]);
        benches!([$( $n_method_name = $n_method_struct ),*], [ $( $n_p_name = $n_p_const ),+ ]);
    };
    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], [ $p_name:ident = $p_const:ident ]) => {
        mod $p_name {
            use super::*;
            benches!([$( $n_method_name = $n_method_struct ),*], $p_name = $p_const);
        }
    };

    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], $p_name:ident = $p_const:ident, [ $row_name:ident = $row_const:ident, $( $n_row_name:ident = $n_row_const:ident ),+ ]) => {
        benches!([ $( $n_method_name = $n_method_struct ),+ ], $p_name = $p_const, [ $row_name = $row_const ]);
        benches!([ $( $n_method_name = $n_method_struct ),+ ], $p_name = $p_const, [ $( $n_row_name = $n_row_const),+ ]);
    };
    ([ $( $n_method_name:ident = $n_method_struct:ident ),+ ], $p_name:ident = $p_const:ident, [ $row_name:ident = $row_const:ident ]) => {
        mod $row_name {
            use super::*;
            benches!([$( $n_method_name = $n_method_struct ),*], $p_name = $p_const, $row_name = $row_const );
        }
    };

    ([$method_name:ident = $method_struct:ident, $( $n_method_name:ident = $n_method_struct:ident ),+], $p_name:ident = $p_const:ident, $row_name:ident = $row_const:ident) => {
        benches!([ $method_name = $method_struct ],          $p_name = $p_const, $row_name = $row_const );
        benches!([$( $n_method_name = $n_method_struct ),*], $p_name = $p_const, $row_name = $row_const );
    };
    ([ $method_name:ident = $method_struct:ident ], $p_name:ident = $p_const:ident, $row_name:ident = $row_const:ident) => {
        #[bench]
        fn $method_name(bencher: &mut Bencher) {
            bench_get_score::<$method_struct>(bencher, $p_const, $row_const);
        }
    };

    ([$method_name:ident = $method_struct:ident, $( $n_method_name:ident = $n_method_struct:ident ),+], $p_name:ident = $p_const:ident) => {
        benches!([ $method_name = $method_struct ],          $p_name = $p_const );
        benches!([$( $n_method_name = $n_method_struct ),*], $p_name = $p_const );
    };
    ([ $method_name:ident = $method_struct:ident ], $p_name:ident = $p_const:ident) => {
        #[bench]
        fn $method_name(bencher: &mut Bencher) {
            bench_total::<$method_struct>(bencher, $p_const);
        }
    };
}

benches!(
    [
        unchecked = Unchecked,
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        short_low_a = PARAMETERS_SHORT_LOW,
        mid_low_a = PARAMETERS_MID_LOW,
        long_low_a = PARAMETERS_LONG_LOW,
        short_high_a = PARAMETERS_SHORT_HIGH,
        mid_high_a = PARAMETERS_MID_HIGH,
        long_high_a = PARAMETERS_LONG_HIGH
    ],
    [a_filled = ROW_A_FILLED]
);

benches!(
    [
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        short_low_b = PARAMETERS_SHORT_LOW,
        mid_low_b = PARAMETERS_MID_LOW,
        long_low_b = PARAMETERS_LONG_LOW,
        short_high_b = PARAMETERS_SHORT_HIGH,
        mid_high_b = PARAMETERS_MID_HIGH,
        long_high_b = PARAMETERS_LONG_HIGH
    ],
    [
        b_one_start = ROW_B_ONE_START,
        c_one_mid = ROW_C_ONE_MID,
        d_one_end = ROW_D_ONE_END
    ]
);

benches!(
    [
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        mid_low_c = PARAMETERS_MID_LOW,
        long_low_c = PARAMETERS_LONG_LOW,
        mid_high_c = PARAMETERS_MID_HIGH,
        long_high_c = PARAMETERS_LONG_HIGH
    ],
    [d_four = ROW_D_FOUR]
);

benches!(
    [
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        long_low_d = PARAMETERS_LONG_LOW,
        long_high_d = PARAMETERS_LONG_HIGH
    ],
    [e_six = ROW_E_SIX]
);

benches!(
    [
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        short_low_e = PARAMETERS_SHORT_LOW,
        mid_low_e = PARAMETERS_MID_LOW,
        long_low_e = PARAMETERS_LONG_LOW,
        short_high_e = PARAMETERS_SHORT_HIGH,
        mid_high_e = PARAMETERS_MID_HIGH,
        long_high_e = PARAMETERS_LONG_HIGH
    ],
    [f_less_empty = ROW_F_LESS_EMPTY, g_empty = ROW_G_EMPTY]
);

benches!(
    [
        checked = Checked,
        naive = Naive,
        double = SwitchDouble,
        triple = SwitchTriple
    ],
    [
        short_low = PARAMETERS_SHORT_LOW,
        mid_low = PARAMETERS_MID_LOW,
        long_low = PARAMETERS_LONG_LOW,
        short_high = PARAMETERS_SHORT_HIGH,
        mid_high = PARAMETERS_MID_HIGH,
        long_high = PARAMETERS_LONG_HIGH
    ]
);
