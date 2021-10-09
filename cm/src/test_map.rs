// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

extern crate test;

use std::sync::{Arc, Mutex, MutexGuard};
use test::{Bencher, black_box};

use lazy_static::lazy_static;

use common::{u_vec, UVec};
use pc_list::PCList;

use crate::{BitArray, CoverageMap};

const STRENGTH: usize = 6;
const PARAMETERS: [usize; 15] = [6, 6, 6, 5, 4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2];
const ROWS: [[usize; 15]; 4] = [
    [3, 3, 4, 3, 3, 2, 1, 0, 1, 0, 0, !0, !0, !0, !0],
    [3, 1, 2, 4, 3, 2, 1, 1, 0, 0, 0, !0, !0, !0, !0],
    [3, 5, 4, 3, 2, 1, 0, 0, 0, 0, 0, !0, !0, !0, !0],
    [!0, 1, !0, 1, 3, 0, !0, 1, !0, 1, !0, !0, !0, !0, !0],
];

const VALUE_CHOICES: usize = 2;

lazy_static! {
    static ref INIT: Arc<Mutex<Option<Init>>> = Arc::new(Mutex::new(None));
}

struct Init {
    pc_list: PCList<usize, u64, STRENGTH>,
    pc_list_len: usize,
    cm: CoverageMap<usize, STRENGTH>,
    locations: UVec<BitArray>,
    scores: UVec<UVec<BitArray>>,
    no_dont_cares: BitArray,
}

impl Init {
    fn borrow_pc(
        &self,
    ) -> (
        &PCList<usize, u64, STRENGTH>,
        usize,
        &CoverageMap<usize, STRENGTH>,
        &UVec<BitArray>,
        &UVec<UVec<BitArray>>,
        BitArray,
    ) {
        (
            &self.pc_list,
            self.pc_list_len,
            &self.cm,
            &self.locations,
            &self.scores,
            self.no_dont_cares,
        )
    }
}

mod test_indices {
    use common::{repeat_strengths, UVec, ValueGenerator};
    use pc_list::PCList;

    use crate::{BIT_SHIFT, BitArray, CoverageMap};

    use super::PARAMETERS;

    macro_rules! call_sub_test {
        ($strength_name:ident, $strength:expr) => {
            #[test]
            fn $strength_name() {
                sub_test::<$strength>();
            }
        };
    }

    repeat_strengths!(call_sub_test);

    fn sub_test<const STRENGTH: usize>() where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
        let at_parameter = PARAMETERS.len() - 2;
        let parameters = UVec::from(PARAMETERS.to_vec());
        let pc_list = PCList::<usize, u64, STRENGTH>::new(PARAMETERS.len());
        let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];

        let mut map = CoverageMap::<usize, STRENGTH>::new(parameters.clone(), &pc_list);
        map.initialise(at_parameter);

        let mut count = 0;
        for (tid, (pc, sizes)) in pc_list
            .pcs
            .iter()
            .take(pc_list_len)
            .zip(map.sizes.iter().skip(1))
            .enumerate()
        {
            for pid in pc.iter() {
                assert!(*pid < at_parameter);
            }

            let mut values = [0; STRENGTH];
            count += 1;
            let generator = ValueGenerator::<usize, STRENGTH>::new(
                &parameters,
                at_parameter,
                pc,
            );
            while generator.next_array(&mut values) {
                count += 1;
            }

            assert_eq!(
                sizes[0] as usize * PARAMETERS[at_parameter],
                count,
                "sizes {} != count {}; tid: {}",
                sizes[0] as usize * PARAMETERS[at_parameter],
                count,
                tid
            );
        }

        assert_eq!(map.value_choices, PARAMETERS[at_parameter]);

        assert_eq!(
            map.uncovered, count,
            "uncovered {} != count {}; vc: {}",
            map.uncovered, count, map.value_choices
        );
        assert_eq!(map.map.len(), 1 + (count >> BIT_SHIFT));

        unsafe {
            map.set_zero_covered();
        }

        assert_eq!(map.uncovered, count - pc_list_len);

        let mut index = 0;
        for (tid, pc) in pc_list.pcs.iter().take(pc_list_len).enumerate() {
            assert!(
                unsafe { !map.set_index(index) },
                "zero check; count {}; index {}; tid {}; next offset: {}",
                count,
                index,
                tid,
                map.sizes[tid + 1][0]
            );

            let mut values = [0; STRENGTH];
            let mut row = [!0; PARAMETERS.len()];
            index += 1;
            let generator = ValueGenerator::<usize, STRENGTH>::new(
                &parameters,
                at_parameter,
                pc,
            );
            while generator.next_array(&mut values) {
                assert!(
                    unsafe { map.set_index(index) },
                    "count {}; index {}; tid {}; next offset: {}",
                    count,
                    index,
                    tid,
                    map.sizes[tid + 1][0]
                );
                if index & 0x0f == 0 {
                    for (pid, value) in pc.iter().zip(values.iter()) {
                        row[*pid] = *value;
                    }
                    assert_eq!(
                        unsafe { map.get_base_index(tid, &pc_list, &row) }.unwrap()
                            + values[STRENGTH - 1] as BitArray,
                        index
                    );
                }
                index += 1;
            }
        }

        assert_eq!(map.uncovered, 0);

        for &a in map.map.iter().take(count >> BIT_SHIFT) {
            assert_eq!(a, BitArray::max_value());
        }
    }
}

fn init_bench() {
    if INIT.lock().unwrap().is_none() {
        let parameter_count = PARAMETERS.len();
        let at_parameter = 11;

        let pc_list = PCList::<usize, u64, STRENGTH>::new(parameter_count);
        let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];
        let mut cm = CoverageMap::<usize, STRENGTH>::new(UVec::from(PARAMETERS.to_vec()), &pc_list);
        cm.initialise(at_parameter);

        let mut locations = UVec::with_capacity(ROWS.len());
        for row in ROWS.iter() {
            let mut location = !0;
            for (parameter_id, &value) in row.iter().enumerate() {
                if value != !0 {
                    location ^= 1 << parameter_id as BitArray;
                }
            }
            for pid in at_parameter..parameter_count {
                assert_eq!(row[pid], !0);
            }
            locations.push(location);
        }

        INIT.lock().unwrap().replace(Init {
            pc_list,
            pc_list_len: pc_list_len,
            cm,
            locations,
            scores: u_vec![UVec::with_capacity(pc_list_len); VALUE_CHOICES],
            no_dont_cares: !((!0) << at_parameter as BitArray),
        });
    }
}

#[inline]
fn check_scores(pc_list_len: usize, scores: &mut UVec<UVec<u64>>, row_id: usize) {
    for score in scores.iter_mut() {
        if row_id != ROWS.len() - 1 {
            assert_eq!(score.len(), pc_list_len);
        } else {
            assert_eq!(score.len(), 6);
        }
        score.clear();
    }
}

#[bench]
fn bench_naive(bencher: &mut Bencher) {
    init_bench();
    let init_guard: MutexGuard<Option<Init>> = INIT.lock().unwrap();
    let (pc_list, pc_list_len, cm, _locations, scores, _no_dont_cares): (
        &PCList<usize, u64, STRENGTH>,
        usize,
        &CoverageMap<usize, STRENGTH>,
        &UVec<BitArray>,
        &UVec<UVec<BitArray>>,
        BitArray,
    ) = init_guard.as_ref().unwrap().borrow_pc();
    let mut scores = scores.clone();

    bencher.iter(|| {
        for (row_id, row) in ROWS.iter().enumerate() {
            unsafe {
                cm.get_high_score(&pc_list, pc_list_len, row, &mut scores);
            }

            check_scores(pc_list_len, &mut scores, row_id);
        }
    });
}

#[bench]
fn bench_masked(bencher: &mut Bencher) {
    init_bench();
    let init_guard: MutexGuard<Option<Init>> = INIT.lock().unwrap();
    let (pc_list, pc_list_len, cm, locations, scores, no_dont_cares): (
        &PCList<usize, u64, STRENGTH>,
        usize,
        &CoverageMap<usize, STRENGTH>,
        &UVec<BitArray>,
        &UVec<UVec<BitArray>>,
        BitArray,
    ) = init_guard.as_ref().unwrap().borrow_pc();
    let mut scores = scores.clone();

    bencher.iter(|| {
        for (row_id, (&dont_care_locations, row)) in locations.iter().zip(ROWS.iter()).enumerate() {
            cm.get_high_score_masked(
                &pc_list,
                pc_list_len,
                row,
                dont_care_locations,
                no_dont_cares,
                &mut scores,
            );

            check_scores(pc_list_len, &mut scores, row_id);
        }
    });
}

#[bench]
fn bench_unchecked_and_naive(bencher: &mut Bencher) {
    init_bench();
    let init_guard: MutexGuard<Option<Init>> = INIT.lock().unwrap();
    let (pc_list, pc_list_len, cm, locations, scores, no_dont_cares): (
        &PCList<usize, u64, STRENGTH>,
        usize,
        &CoverageMap<usize, STRENGTH>,
        &UVec<BitArray>,
        &UVec<UVec<BitArray>>,
        BitArray,
    ) = init_guard.as_ref().unwrap().borrow_pc();
    let mut scores = scores.clone();

    bencher.iter(|| {
        for (row_id, (&dont_care_locations, row)) in locations.iter().zip(ROWS.iter()).enumerate() {
            if no_dont_cares & dont_care_locations == 0 {
                unsafe {
                    cm.get_high_score_masked_unchecked(&pc_list, pc_list_len, row, &mut scores);
                }
            } else {
                unsafe {
                    cm.get_high_score(&pc_list, pc_list_len, row, &mut scores);
                }
            }

            check_scores(pc_list_len, &mut scores, row_id);
        }
    });
}

#[bench]
fn bench_unchecked_and_naive_count(bencher: &mut Bencher) {
    init_bench();
    let init_guard: MutexGuard<Option<Init>> = INIT.lock().unwrap();
    let (pc_list, pc_list_len, cm, locations, scores, no_dont_cares): (
        &PCList<usize, u64, STRENGTH>,
        usize,
        &CoverageMap<usize, STRENGTH>,
        &UVec<BitArray>,
        &UVec<UVec<BitArray>>,
        BitArray,
    ) = init_guard.as_ref().unwrap().borrow_pc();
    let mut scores = scores.clone();

    bencher.iter(|| {
        for (row_id, (&dont_care_locations, row)) in locations.iter().zip(ROWS.iter()).enumerate() {
            let dont_care_count = ((no_dont_cares & dont_care_locations) as usize).count_ones();
            if dont_care_count == 0 {
                unsafe {
                    cm.get_high_score_masked_unchecked(&pc_list, pc_list_len, row, &mut scores);
                }
            } else {
                unsafe {
                    cm.get_high_score(&pc_list, pc_list_len, row, &mut scores);
                }
            }

            check_scores(pc_list_len, &mut scores, row_id);
        }
    });
}

#[bench]
fn bench_count_zeros(bencher: &mut Bencher) {
    let a: u64 = black_box(0x45a400ad);
    let b: u64 = black_box(0x8ad8f400);
    bencher.iter(|| {
        assert_eq!((a & b).count_ones(), 1);
    });
}
