// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This benchmark tests the speed of various score filtering methods.

use test::Bencher;

use cm::{BIT_MASK, BIT_SHIFT, BitArray, CoverageMap};
use common::{u_vec, UVec};

use crate::unconstrained::bench_horizontal::TestData;
use crate::unconstrained::bench_init::PARAMETERS;

struct FilterTestData {
    td: TestData,
    row_previous: usize,
    row_next: usize,
    value: usize,
    scores_previous: UVec<UVec<BitArray>>,
    scores_next: UVec<UVec<BitArray>>,
    filtered: usize,
}

impl FilterTestData {
    fn low_filter() -> Self {
        Self::new(1, 1)
    }

    fn high_filter() -> Self {
        Self::new(2, 21)
    }

    fn new(row_next: usize, filtered: usize) -> Self {
        let td = TestData::new(&PARAMETERS);
        let scores_previous = u_vec![UVec::with_capacity(td.pc_list_len); td.value_choices as usize];
        let scores_next = scores_previous.clone();

        let mut result = Self {
            td,
            row_previous: 0,
            row_next,
            value: 0,
            scores_previous,
            scores_next,
            filtered,
        };

        unsafe { result.td.coverage_map.get_high_score(&result.td.pc_list, result.td.pc_list_len, result.td.mca.array[result.row_previous].as_slice(), &mut result.scores_previous) };
        unsafe { result.td.coverage_map.get_high_score(&result.td.pc_list, result.td.pc_list_len, result.td.mca.array[result.row_next].as_slice(), &mut result.scores_next) };
        unsafe { result.td.coverage_map.set_indices(&result.scores_previous[result.value]) };

        result
    }
}

#[inline]
fn filter_scores_to_zero(new_vec: &mut UVec<u64>, old_vec: &UVec<u64>, _coverage_map: &CoverageMap<u8, 6>) -> usize {
    let mut new_iter = new_vec.iter_mut();
    let mut old_iter = old_vec.iter();
    let mut result = 0;
    while let (Some(mut new), Some(mut old)) = (new_iter.next(), old_iter.next()) {
        while *new != *old {
            while *new < *old {
                if let Some(temp) = new_iter.next() {
                    new = temp;
                } else { return result; }
            }

            while *new > *old {
                if let Some(temp) = old_iter.next() {
                    old = temp;
                } else { return result; }
            }
        }

        *new = 0;
        result += 1;
    }

    result
}

#[inline]
fn filter_scores_remove(new_vec: &mut UVec<u64>, old_vec: &UVec<u64>, _coverage_map: &CoverageMap<u8, 6>) -> usize {
    let mut new_index = 0;
    let mut old_iter = old_vec.iter();
    let mut result = 0;
    while let (Some(mut new), Some(mut old)) = (new_vec.get_mut(new_index), old_iter.next()) {
        while *new != *old {
            while *new < *old {
                new_index += 1;
                if let Some(temp) = new_vec.get_mut(new_index) {
                    new = temp;
                } else { return result; }
            }

            while *new > *old {
                if let Some(temp) = old_iter.next() {
                    old = temp;
                } else { return result; }
            }
        }

        new_vec.remove(new_index);
        result += 1;
    }

    result
}

#[inline]
fn filter_scores_cm_remove(new_vec: &mut UVec<u64>, _old_vec: &UVec<u64>, coverage_map: &CoverageMap<u8, 6>) -> usize {
    let previous_size = new_vec.len();
    new_vec.retain(|index| {
        let array_id = *index as usize >> BIT_SHIFT;
        debug_assert!(array_id < coverage_map.map.len());
        let array = coverage_map.map[array_id];
        let index = 1 << (*index & BIT_MASK);
        array & index == 0
    });
    previous_size - new_vec.len()
}

#[inline]
fn filter_scores_cm_zero(new_vec: &mut UVec<u64>, _old_vec: &UVec<u64>, coverage_map: &CoverageMap<u8, 6>) -> usize {
    let mut result = 0;
    for index in new_vec.iter_mut() {
        if {
            let array_id = *index as usize >> BIT_SHIFT;
            debug_assert!(array_id < coverage_map.map.len());
            let array = coverage_map.map[array_id];
            let index = 1 << (*index & BIT_MASK);
            array & index != 0
        } {
            *index = 0;
            result += 1;
        }
    }
    result
}

#[bench]
fn index_filter_to_zero_high(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::high_filter(), filter_scores_to_zero, false)
}

#[bench]
fn index_filter_to_zero_low(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::low_filter(), filter_scores_to_zero, false)
}

#[bench]
fn index_filter_remove_high(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::high_filter(), filter_scores_remove, true)
}

#[bench]
fn index_filter_remove_low(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::low_filter(), filter_scores_remove, true)
}

#[bench]
fn cm_filter_zero_high(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::high_filter(), filter_scores_cm_zero, false);
}

#[bench]
fn cm_filter_zero_low(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::low_filter(), filter_scores_cm_zero, false);
}

#[bench]
fn cm_filter_remove_high(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::high_filter(), filter_scores_cm_remove, true);
}

#[bench]
fn cm_filter_remove_low(bencher: &mut Bencher) {
    bench_filter(bencher, FilterTestData::low_filter(), filter_scores_cm_remove, true);
}

#[inline]
fn bench_filter(bencher: &mut Bencher, ftd: FilterTestData, filter: impl Fn(&mut UVec<BitArray>, &UVec<BitArray>, &CoverageMap<u8, 6>) -> usize, removes: bool) {
    bencher.iter(|| {
        let mut new_scores: UVec<BitArray> = ftd.scores_next[ftd.value].clone();
        let previous_len = new_scores.len();
        let score = filter(&mut new_scores, &ftd.scores_previous[ftd.value], &ftd.td.coverage_map);
        assert_eq!(previous_len, if removes { new_scores.len() + ftd.filtered } else { new_scores.len() });
        assert_eq!(score, ftd.filtered);
    });
}

#[bench]
fn filtered_cover_ignore_high(bencher: &mut Bencher) {
    if cfg!(not(debug_assertions)) {
        filtered_cover_ignore(bencher, FilterTestData::high_filter())
    }
}

#[bench]
fn filtered_cover_ignore_low(bencher: &mut Bencher) {
    if cfg!(not(debug_assertions)) {
        filtered_cover_ignore(bencher, FilterTestData::low_filter())
    }
}

#[bench]
fn filtered_cover_skip_high(bencher: &mut Bencher) {
    if cfg!(not(debug_assertions)) {
        filtered_cover_skip(bencher, FilterTestData::high_filter())
    }
}

#[bench]
fn filtered_cover_skip_low(bencher: &mut Bencher) {
    if cfg!(not(debug_assertions)) {
        filtered_cover_skip(bencher, FilterTestData::low_filter())
    }
}

#[bench]
fn filtered_cover_remove_high(bencher: &mut Bencher) {
    filtered_cover_remove(bencher, FilterTestData::high_filter())
}

#[bench]
fn filtered_cover_remove_low(bencher: &mut Bencher) {
    filtered_cover_remove(bencher, FilterTestData::low_filter())
}

#[inline]
fn filtered_cover_ignore(bencher: &mut Bencher, ftd: FilterTestData) {
    let mut scores = ftd.scores_next[ftd.value].clone();
    filter_scores_cm_zero(&mut scores, &ftd.scores_previous[ftd.value], &ftd.td.coverage_map);
    let mut cm = ftd.td.coverage_map.clone();
    bencher.iter(|| {
        unsafe { cm.set_indices_sub(&scores); }
    });
}

#[inline]
fn filtered_cover_skip(bencher: &mut Bencher, ftd: FilterTestData) {
    let mut scores = ftd.scores_next[ftd.value].clone();
    filter_scores_cm_zero(&mut scores, &ftd.scores_previous[ftd.value], &ftd.td.coverage_map);
    let mut cm = ftd.td.coverage_map.clone();
    bencher.iter(|| {
        unsafe {
            for &score in scores.iter() {
                if score != 0 {
                    cm.set(score);
                }
            }
        }
    });
}

#[inline]
fn filtered_cover_remove(bencher: &mut Bencher, ftd: FilterTestData) {
    let mut scores = ftd.scores_next[ftd.value].clone();
    filter_scores_cm_remove(&mut scores, &ftd.scores_previous[ftd.value], &ftd.td.coverage_map);
    let mut cm = ftd.td.coverage_map.clone();
    bencher.iter(|| {
        unsafe { cm.set_indices(&scores); }
    });
}

#[bench]
fn get_score_plain(bencher: &mut Bencher) {
    let mut td = TestData::new(&PARAMETERS);
    unsafe { td.coverage_map.get_high_score(&td.pc_list, td.pc_list_len, td.mca.array[0].as_slice(), &mut td.scores) };
    bencher.iter(|| {
        for scores in td.scores.iter_mut() {
            scores.clear();
        }
        unsafe { td.coverage_map.get_high_score(&td.pc_list, td.pc_list_len, td.mca.array[1].as_slice(), &mut td.scores) };
    });
}

#[bench]
fn get_score_chosen(bencher: &mut Bencher) {
    let mut td = TestData::new(&PARAMETERS);
    unsafe { td.coverage_map.get_high_score(&td.pc_list, td.pc_list_len, td.mca.array[0].as_slice(), &mut td.scores) };
    unsafe { td.coverage_map.set_indices(&td.scores[0]) };
    bencher.iter(|| {
        for scores in td.scores.iter_mut() {
            scores.clear();
        }
        unsafe { td.coverage_map.get_high_score(&td.pc_list, td.pc_list_len, td.mca.array[1].as_slice(), &mut td.scores) };
    });
}
