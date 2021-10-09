// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use test::Bencher;

use cm::CoverageMap;
use common::{Number, u_vec, UVec};
use mca::MCA;
use pc_list::PCList;

use crate::threads_common::split;
use crate::unconstrained::bench_init::PARAMETERS;

const THREAD_COUNT: usize = 2;

pub(crate) const BASIC_MCA: [[u8; PARAMETERS.len()]; 4] = [
    [1, 1, 3, 1, 2, 2, !0, 1, !0, 2, 1, 2, 1, !0, !0],
    [1, 1, 1, 2, 1, 2, 1, !0, 1, 2, 1, 1, !0, !0, !0],
    [1, 2, 2, 1, 2, 2, 2, 1, !0, 1, !0, 2, 1, !0, !0],
    [2, 1, 2, 2, 1, 1, 1, 1, 2, !0, 2, !0, 1, !0, !0],
];

pub(crate) const BASIC_MCA_ALT: [[u8; PARAMETERS.len()]; 4] = [
    [!0, !0, !0, !0, !0, !0, 1, 1, 0, 2, 1, 2, 1, !0, !0],
    [!0, !0, !0, 2, 1, 2, 1, 1, 1, 2, 1, 1, 2, !0, !0],
    [!0, !0, 2, 1, 2, 2, 2, 1, 1, 1, 0, 2, 1, !0, !0],
    [!0, 1, !0, 2, 1, 1, 1, 1, 2, 0, 2, !0, 1, !0, !0],
];

#[derive(Clone)]
pub(crate) struct TestData {
    pub(crate) mca: MCA<u8, u64>,
    pub(crate) at_parameter: usize,
    pub(crate) value_choices: u8,
    pub(crate) pc_list: PCList<u8, u64, 6>,
    pub(crate) pc_list_len: usize,
    pub(crate) coverage_map: CoverageMap<u8, 6>,
    pub(crate) scores: UVec<UVec<u64>>,
}

impl TestData {
    pub(crate) fn new(parameters: &[u8; PARAMETERS.len()]) -> Self { Self::new_with_mca(parameters, &BASIC_MCA) }
    pub(crate) fn new_with_mca(parameters: &[u8; PARAMETERS.len()], basic_mca: &[[u8; PARAMETERS.len()]; 4]) -> Self {
        let mca = get_basic_mca(basic_mca);
        let at_parameter = parameters.len() - 2;
        let value_choices = parameters[at_parameter];

        let pc_list = PCList::new(parameters.len());
        let pc_list_len = pc_list.sizes[at_parameter - 6];

        let mut coverage_map = CoverageMap::new(UVec::from(parameters.to_vec()), &pc_list);
        coverage_map.initialise(at_parameter);
        unsafe { coverage_map.set_zero_covered(); }
        let scores = u_vec![UVec::with_capacity(pc_list_len); value_choices as usize];

        TestData {
            mca,
            at_parameter,
            value_choices,
            pc_list,
            pc_list_len,
            coverage_map,
            scores,
        }
    }
}


fn get_basic_mca(basic_mca: &[[u8; PARAMETERS.len()]; 4]) -> MCA<u8, u64> {
    let mut array = UVec::with_capacity(basic_mca.len());
    for row in basic_mca.iter() {
        array.push(UVec::from(row.to_vec()));
    }

    let mut dont_care_locations = UVec::with_capacity(basic_mca.len());
    for row in array.iter() {
        let mut locations = 0;
        for value in row.iter().rev() {
            if *value != u8::dont_care() {
                locations += 1;
            }
            locations <<= 1;
        }
        dont_care_locations.push(locations);
    }

    MCA { array, dont_care_locations, vertical_extension_rows: UVec::with_capacity(0), new_row: UVec::with_capacity(0) }
}

#[bench]
fn score_high_original(bencher: &mut Bencher) { score_original(bencher, &BASIC_MCA); }

#[bench]
fn score_high_0(bencher: &mut Bencher) { score_partial(bencher, 0, &BASIC_MCA); }

#[bench]
fn score_high_1(bencher: &mut Bencher) { score_partial(bencher, 1, &BASIC_MCA); }

#[bench]
fn score_high_all(bencher: &mut Bencher) { score_all(bencher, &BASIC_MCA); }

#[bench]
fn score_low_original(bencher: &mut Bencher) { score_original(bencher, &BASIC_MCA_ALT); }

#[bench]
fn score_low_0(bencher: &mut Bencher) { score_partial(bencher, 0, &BASIC_MCA_ALT); }

#[bench]
fn score_low_1(bencher: &mut Bencher) { score_partial(bencher, 1, &BASIC_MCA_ALT); }

#[bench]
fn score_low_all(bencher: &mut Bencher) { score_all(bencher, &BASIC_MCA_ALT); }

fn score_original(bencher: &mut Bencher, basic_mca: &[[u8; PARAMETERS.len()]; 4]) {
    let mut t = TestData::new_with_mca(&PARAMETERS, basic_mca);
    let start = 0;
    let end = t.pc_list_len;
    bencher.iter(|| score_part(&mut t, start, end));
}

fn score_all(bencher: &mut Bencher, basic_mca: &[[u8; PARAMETERS.len()]; 4]) {
    let mut t = TestData::new_with_mca(&PARAMETERS, basic_mca);
    let parts = [
        split(THREAD_COUNT, 0, t.pc_list_len),
        split(THREAD_COUNT, 1, t.pc_list_len),
    ];

    bencher.iter(|| {
        for &(start, end) in parts.iter() {
            score_part(&mut t, start, end);
        }
    });
}

fn score_partial(bencher: &mut Bencher, thread_id: usize, basic_mca: &[[u8; PARAMETERS.len()]; 4]) {
    let mut t = TestData::new_with_mca(&PARAMETERS, basic_mca);
    let (start, end) = split(THREAD_COUNT, thread_id, t.pc_list_len);

    bencher.iter(|| score_part(&mut t, start, end));
}

#[inline(never)]
fn score_part(t: &mut TestData, start: usize, end: usize) {
    for score in t.scores.iter_mut() {
        unsafe { score.set_len(0); }
    }

    t.coverage_map.get_high_score_masked_triple_sub(&t.pc_list, t.mca.array[0].as_slice(), t.mca.dont_care_locations[0], !((!0) << t.at_parameter), &mut t.scores, start, end);
}


#[test]
fn test_scores() {
    let mut t = TestData::new_with_mca(&PARAMETERS, &BASIC_MCA);
    let mut scores = u_vec![u_vec![0; t.value_choices as usize]; t.mca.array.len()];

    for thread_id in 0..THREAD_COUNT {
        let (start, end) = split(THREAD_COUNT, thread_id, t.pc_list_len);

        for (row_id, row_scores) in scores.iter_mut().enumerate() {
            for score in t.scores.iter_mut() {
                score.clear();
            }

            unsafe { t.coverage_map.get_high_score_sub(&t.pc_list, t.mca.array[row_id].as_slice(), &mut t.scores, start, end); }

            for (score, indices) in row_scores.iter_mut().zip(t.scores.iter()) {
                *score += indices.len();
            }
        }
    }

    for (row, row_scores) in t.mca.array.iter().zip(scores.iter()) {
        for score in t.scores.iter_mut() {
            score.clear();
        }

        unsafe { t.coverage_map.get_high_score(&t.pc_list, t.pc_list_len, row.as_slice(), &mut t.scores); }

        for (score, indices) in row_scores.iter().zip(t.scores.iter()) {
            assert_eq!(*score, indices.len());
        }
    }
}


#[bench]
fn bench_split_fn(bencher: &mut Bencher) {
    let t = TestData::new_with_mca(&PARAMETERS, &BASIC_MCA);
    let thread_id = test::black_box(0);
    bencher.iter(|| {
        let (start, end) = split(THREAD_COUNT, thread_id, t.pc_list_len);
        assert_ne!(start, end)
    });
}
