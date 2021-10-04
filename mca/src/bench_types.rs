// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

extern crate test;

use test::{Bencher, black_box};

use common::{u_vec, UVec};

const ONES: [usize; 16] = [
    14,
    15,
    18,
    24,
    32,
    37,
    54,
    56,
    63,
    65,
    68,
    69,
    75,
    78,
    89,
    95,
];
const ONE_COUNT: usize = ONES.len() * 10;
const P_LEN: usize = 12;
const MCA_LEN: usize = 100;
const SEARCHES: [usize; 30] = [2, 3, 6, 8, 9, 12, 15, 16, 19, 23, 25, 27, 29, 30, 33, 35, 39, 40, 55, 56, 57, 58, 70, 72, 73, 81, 82, 87, 88, 95];
const SEARCH_ONE_COUNT: usize = 45;

fn mca_one_dim() -> UVec<u8> {
    let mut result: UVec<u8> = u_vec![0; P_LEN * MCA_LEN];
    for one in ONES.iter() {
        for m in 0..10 {
            result[m * 100 + one] = 1;
        }
    }
    result
}

fn mca_two_dims_dyn() -> UVec<UVec<u8>> {
    let mut result = u_vec![u_vec![0; P_LEN]; MCA_LEN];
    for one in ONES.iter() {
        for m in 0..10 {
            let index = m * 100 + one;
            result[index / P_LEN][index % P_LEN] = 1;
        }
    }
    result
}

fn mca_two_dims_stat() -> UVec<[u8; P_LEN]> {
    let mut result = u_vec![[0; P_LEN]; MCA_LEN];
    for one in ONES.iter() {
        for m in 0..10 {
            let index = m * 100 + one;
            result[index / P_LEN][index % P_LEN] = 1;
        }
    }
    result
}

#[bench]
fn bench_two_dims_dyn_read_all(bencher: &mut Bencher) {
    let mca = mca_two_dims_dyn();

    bencher.iter(|| {
        let mut count = 0;

        for row in mca.iter() {
            for cell in row.iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, ONE_COUNT);
    })
}

#[bench]
fn bench_two_dims_stat_read_all(bencher: &mut Bencher) {
    let mca = mca_two_dims_stat();

    bencher.iter(|| {
        let mut count = 0;

        for row in mca.iter() {
            for cell in row.iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, ONE_COUNT);
    })
}

#[bench]
fn bench_one_dim_dyn_read_all(bencher: &mut Bencher) {
    let mca = mca_one_dim();

    bencher.iter(|| {
        let mut count = 0;

        for cell in mca.iter() {
            if *cell == 1 {
                count += 1;
            }
        }

        assert_eq!(count, ONE_COUNT);
    })
}

#[bench]
fn bench_one_dim_stat_read_all(bencher: &mut Bencher) {
    let mca = mca_one_dim();

    unsafe {
        bencher.iter(|| {
            let mut count = 0;

            for chunk in mca.as_chunks_unchecked::<P_LEN>() {
                for cell in chunk {
                    if *cell == 1 {
                        count += 1;
                    }
                }
            }

            assert_eq!(count, ONE_COUNT);
        })
    }
}

#[bench]
fn bench_two_dims_dyn_search(bencher: &mut Bencher) {
    let mca = mca_two_dims_dyn();
    bencher.iter(|| {
        let mut count = 0;

        for search in SEARCHES.iter() {
            for cell in mca[*search].iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, SEARCH_ONE_COUNT);
    })
}


#[bench]
fn bench_two_dims_stat_search(bencher: &mut Bencher) {
    let mca = mca_two_dims_stat();
    bencher.iter(|| {
        let mut count = 0;

        for search in SEARCHES.iter() {
            for cell in mca[*search].iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, SEARCH_ONE_COUNT);
    })
}

#[bench]
fn bench_one_dim_stat_search(bencher: &mut Bencher) {
    let mca = mca_one_dim();
    bencher.iter(|| {
        let mut count = 0;

        for search in SEARCHES.iter() {
            for cell in mca[search * P_LEN..(search + 1) * P_LEN].iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, SEARCH_ONE_COUNT);
    })
}

#[bench]
fn bench_one_dim_dyn_search(bencher: &mut Bencher) {
    let mca = mca_one_dim();
    let p_len = black_box(P_LEN);
    bencher.iter(|| {
        let mut count = 0;

        for search in SEARCHES.iter() {
            for cell in mca[search * p_len..(search + 1) * p_len].iter() {
                if *cell == 1 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, SEARCH_ONE_COUNT);
    })
}
